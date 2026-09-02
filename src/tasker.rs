use std::{str::FromStr, sync::OnceLock};

use chrono::{Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use reqwest::Response;

use crate::env;

/// Records one action for a user against the tasker service at `host`.
///
/// `POST /actions/{type}/{user_id}` names the player in the *path* and carries
/// no body, so the `X-Internal-Secret` header is the only thing separating a
/// real caller from anyone who can reach the host. Without it, whoever finds the
/// URL can complete any player's tasks and collect the rewards.
pub async fn new_action_with_host(
    action_type: &str,
    user_id: i64,
    client: &reqwest::Client,
    host: String,
) -> Result<Response, reqwest::Error> {

    client
        .post(format!("{host}/actions/{action_type}/{user_id}"))
        .header("X-Internal-Secret", internal_secret())
        .send()
        .await
}

static TASKER: OnceLock<String> = OnceLock::new();

/// Railway's private address for the tasker service, used when `TASKER_HOST`
/// says nothing.
///
/// Private rather than public because of what the route above looks like: the
/// player is in the path, so keeping the traffic off the public internet is
/// worth more than one URL that works everywhere. Port 8080 is what tasker
/// binds, on `[::]`, which is what Railway's IPv6-only internal DNS needs.
pub const DEFAULT_HOST: &str = "http://tasker.railway.internal:8080";

/// Where this deployment's tasker lives.
///
/// Falls back to `DEFAULT_HOST` rather than panicking on a missing
/// `TASKER_HOST`: a service deployed beside tasker on Railway should not have to
/// name it, and taking the whole process down over an unset variable is a poor
/// trade for a task counter. Public so a caller can report it at boot.
pub fn host() -> &'static str {
    TASKER.get_or_init(|| {
        match env::get("TASKER_HOST") {
            Ok(h) if !h.trim().is_empty() => h.trim().trim_end_matches('/').to_string(),
            _ => DEFAULT_HOST.to_string(),
        }
    })
}

static SECRET: OnceLock<String> = OnceLock::new();

/// The shared secret proving a call to `/actions` came from one of our own
/// services, matched against the tasker deployment's own `INTERNAL_SECRET`.
///
/// Read from the environment rather than taken as an argument so that adding
/// authentication did not have to touch every call site in every project. Empty
/// when unset, which the tasker rejects — a loud 400 at the tasker, rather than
/// a silently unauthenticated write.
fn internal_secret() -> &'static str {
    SECRET.get_or_init(|| {
        let secret = env::get("INTERNAL_SECRET").unwrap_or_default();
        if secret.is_empty() {
            println!("INTERNAL_SECRET is not set — tasker will reject every action");
        }
        secret
    })
}

/// Records one action for a user against this deployment's tasker.
pub async fn new_action(
    action_type: &str,
    user_id: i64,
    client: &reqwest::Client
) -> Result<Response, reqwest::Error> {

    new_action_with_host(action_type, user_id, client, host().to_string()).await
}

/// The tables every tasker deployment needs, as one idempotent script.
///
/// Tasker is one codebase deployed once per project, each instance pointed at
/// that project's own database, so this schema is created N times over N
/// databases and has no project column anywhere — the deployment topology is
/// the scoping. It lives here, beside the queries below that read these tables,
/// so that the projects hosting them do not each keep a copy that drifts.
///
/// Safe to run repeatedly. Apply it with `migrate`.
pub const SCHEMA: &str = r#"
-- Labels, not ordinals, are the contract: sqlx maps these to the tasker
-- service's Rust enums by name, so the declaration order is free but every
-- label must match a variant's snake_case spelling. A label missing here fails
-- at query time inside tasker, not at deploy time.
DO $$ BEGIN
    CREATE TYPE task_type AS ENUM (
        'like_video', 'subscribe_social', 'comment_social', 'share_social',
        'premium_social', 'connect_social', 'launch_app', 'log_in', 'check_in',
        'invite', 'link', 'recharge', 'status_social', 'watch_ad', 'beta',
        'deposit', 'withdraw', 'swap', 'connect_wallet', 'custom'
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

DO $$ BEGIN
    CREATE TYPE task_category AS ENUM (
        'daily', 'weekly', 'monthly', 'seasonal', 'timed',
        'social', 'partner', 'special', 'custom'
    );
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- One row per player who has ever done anything task-shaped.
--
-- `completed` maps a period key to the task names finished in it:
--   {"2026-09-02": ["daily_play_3"], "2026-W36": ["weekly_publish"],
--    "permanent": ["first_game"]}
-- The key comes from the task's category, which is what makes a daily task
-- repeat and a special task not.
--
-- No foreign key to a users table: the row is created by the first action
-- posted for an id, and tasker is a separate service that should not fail on
-- the host project's insert ordering.
CREATE TABLE IF NOT EXISTS taskers (
    id BIGINT NOT NULL PRIMARY KEY,
    last_login TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,

    completed JSONB DEFAULT '{}'::JSONB,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- The task catalogue, edited by hand.
--
-- `name` is the identity, not `id`: it is what lands in `taskers.completed` and
-- what rewards are looked up by. Renaming a task after anyone has completed it
-- orphans their completion and they can earn it again.
CREATE TABLE IF NOT EXISTS tasker_tasks (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,

    type task_type NOT NULL DEFAULT 'like_video',
    category task_category NOT NULL DEFAULT 'daily',

    rewards JSONB NOT NULL DEFAULT '{}'::JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Rewards are read by name on every completion; without this that is a seq
-- scan. Unique because two rows sharing a name make that lookup fail outright,
-- and because it is what lets a seed file upsert.
CREATE UNIQUE INDEX IF NOT EXISTS tasker_tasks_name ON tasker_tasks (name);

-- The event log every counted task is measured against.
--
-- Append-only and never deduplicated: progress is count(*) over a time window,
-- so a daily task counts today's rows, a weekly one this week's, and a special
-- one every row ever written. Deleting from here silently un-completes tasks
-- that were in progress.
CREATE TABLE IF NOT EXISTS tasker_actions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    -- The real constraint on every project's action vocabulary. Tasker rejects
    -- a longer name with a 400 rather than truncating it, so an overlong name
    -- is a task that never progresses.
    action_type VARCHAR(16) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Every read of this table is (user, type, window). A two-column index leaves
-- the type as a filter over every action the player has ever taken.
CREATE INDEX IF NOT EXISTS idx_tasker_actions_user_type_created
    ON tasker_actions (user_id, action_type, created_at);

-- Rhai source, keyed by the slug a task's `metadata.script` names.
CREATE TABLE IF NOT EXISTS tasker_scripts (
    id SERIAL PRIMARY KEY,
    slug TEXT UNIQUE NOT NULL,
    code TEXT NOT NULL,

    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Ad-network postbacks, keyed by the network's own event id so a duplicate
-- callback cannot pay twice.
CREATE TABLE IF NOT EXISTS tasker_ads (
    ymid TEXT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payout NUMERIC(20, 10) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tasker_ads_user ON tasker_ads (user_id);

CREATE TABLE IF NOT EXISTS tasker_campaigns (
    id SERIAL PRIMARY KEY,

    name VARCHAR(30) NOT NULL,
    tasks JSONB NOT NULL DEFAULT '{}'::JSONB,

    rewards JSONB NOT NULL DEFAULT '{}'::JSONB,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
"#;

/// Applies `SCHEMA` to `pool`.
///
/// Sent as one multi-statement string rather than split and executed
/// individually, which is what keeps the `DO $$ ... $$` blocks intact — a naive
/// split on semicolons cuts them in half. `sqlx::raw_sql` runs the batch in a
/// single implicit transaction, so a failure leaves nothing half-applied.
///
/// Idempotent, but still a deploy step rather than something to run on boot:
/// a rolling restart would otherwise have N instances running DDL at once.
pub async fn migrate(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(SCHEMA).execute(pool).await?;
    Ok(())
}

pub async fn check_weekly_task(pool: &sqlx::PgPool, action_type: &str, user_id: i64) -> i64 {
    let from_timestamp = get_current_monday_timestamp();

    let to_timestamp = from_timestamp + (7 * 24 * 60 * 60) - 1;
    get_sum_in_range(
        pool,
        action_type,
        user_id,
        from_timestamp,
        to_timestamp
    ).await
}

pub fn get_current_monday_timestamp() -> i64 {
    let now = Utc::now();
    // Days since last Monday (Mon=0, Tue=1, ..., Sun=6 in ISO)
    let days_from_monday = now.weekday().num_days_from_monday();
    
    // Get to Monday 00:00:00
    let monday = (now - chrono::Duration::days(days_from_monday as i64))
        .date_naive()
        .and_time(NaiveTime::MIN); // 00:00:00
        
    Utc.from_local_datetime(&monday).unwrap().timestamp()
}

pub fn is_in_range(now: &str, starts_at: &str, ends_at: &str) -> bool {
    // Standard ISO 8601 strings (YYYY-MM-DD) are lexicographically comparable
    now >= starts_at && now <= ends_at
}

pub async fn get_sum_in_range(
    pool: &sqlx::PgPool,
    action_type: &str,
    user_id: i64,
    from_ts: i64,
    to_ts: i64
) -> i64 {
    // Convert Unix timestamps to Timestamptz for Postgres
    // let start = Utc.timestamp_opt(from_ts, 0).unwrap();
    // let end = Utc.timestamp_opt(to_ts, 0).unwrap();

    let Some(start) = Utc.timestamp_opt(from_ts, 0).single() else {
        println!("Invalid from_ts: {}", from_ts);
        return 0;
    };

    let Some(end) = Utc.timestamp_opt(to_ts, 0).single() else {
        println!("Invalid to_ts: {}", to_ts);
        return 0;
    };

    if start >= end {
        return 0;
    }

    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*) as count 
        FROM tasker_actions 
        WHERE action_type = $1 
            AND user_id = $2
            AND created_at >= $3 
            AND created_at < $4
        "#,
    )
    .bind(action_type)
    .bind(user_id)
    .bind(start)
    .bind(end)

    .fetch_one(pool)
    .await;

    match result {
        Ok(count) => count,
        Err(err) => {
            println!("Database query error: {:?}", err);
            0
        }
    }
}

pub async fn get_actions_by_type(
    pool: &sqlx::PgPool,
    action_type: &str,
    user_id: i64,
    day_str: &str
) -> i64 {
    // let day = NaiveDate::from_str(day_str)
    //     .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

    let day = match NaiveDate::from_str(day_str) {
        Ok(day) => day,
        Err(err) => {
            println!("Invalid day_str {:?}: {:?}", day_str, err);
            return 0;
        }
    };

    let result = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tasker_actions 
        WHERE action_type = $1 
            AND user_id = $2
            -- AND created_at::date = $3::date
            AND created_at >= ($3::date AT TIME ZONE 'UTC')
            AND created_at < (($3::date + INTERVAL '1 day') AT TIME ZONE 'UTC')
        "#,
    )
    .bind(action_type)
    .bind(user_id)
    .bind(day)

    .fetch_one(pool)
    .await;

     match result {
        Ok(count) => count,
        Err(err) => {
            println!("Database query error: {:?}", err);
            0
        }
    }
}

pub async fn check_monthly_task(pool: &sqlx::PgPool, action_type: &str, user_id: i64) -> i64 {
    let now = Utc::now();
    
    // Start of current month: Year-Month-01 00:00:00
    let start_of_month = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .unwrap();
    let from_ts = start_of_month.timestamp();

    // Logic for start of NEXT month
    let next_month_start = if now.month() == 12 {
        Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0).unwrap()
    } else {
        Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0).unwrap()
    };
    
    // End of current month: Next month start - 1 second
    let to_ts = next_month_start.timestamp() - 1;

    get_sum_in_range(pool, action_type, user_id, from_ts, to_ts).await
}

pub async fn check_special_task(pool: &sqlx::PgPool, action_type: &str, user_id: i64) -> i64 {
    // For Special/Social/Partner tasks, we usually check the Lifetime Sum
    // We can use a range from Unix Epoch (0) to now.
    let to_ts = Utc::now().timestamp();
    
    get_sum_in_range(pool, action_type, user_id, 0, to_ts).await
}