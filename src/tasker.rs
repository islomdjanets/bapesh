use reqwest::Response;

use crate::env;

pub async fn new_action_with_host(
    action_type: &str,
    user_id: i64,
    client: &reqwest::Client,
    host: String,
) -> Result<Response, reqwest::Error> {

    // let host = env::get("TASKER_HOST")
    //      .expect("TASKER_HOST IS NOT SETUP");

    client
        .post(format!("{host}/actions/{action_type}/{user_id}"))
        .send()
        .await
        // let now = parse_from_rfc3339(
        //     &now().to_string()
        // ).unwrap();
        // // now.setHours(0, 0, 0, 0);
        // let timestamp = to_rfc3339(now);

        // const key = "actions";
        // // const actions = DATA.$(key).get(timestamp) || [];
        // await DATA.update_map(key, timestamp, action_type);
        // // console.log(key, DATA.$(key).get(timestamp));


        // format: {action_type}:{hour-minute}

}

pub async fn new_action(
    action_type: &str,
    user_id: i64,
    client: &reqwest::Client
) -> Result<Response, reqwest::Error> {

    let host = env::get("TASKER_HOST")
        .expect("TASKER_HOST IS NOT SETUP");

    client
        .post(format!("{host}/actions/{action_type}/{user_id}"))
        .send()
        .await
        // let now = parse_from_rfc3339(
        //     &now().to_string()
        // ).unwrap();
        // // now.setHours(0, 0, 0, 0);
        // let timestamp = to_rfc3339(now);

        // const key = "actions";
        // // const actions = DATA.$(key).get(timestamp) || [];
        // await DATA.update_map(key, timestamp, action_type);
        // // console.log(key, DATA.$(key).get(timestamp));

    

        // format: {action_type}:{hour-minute}

        // Ok(())
}

async fn check_weekly_task(pool: &sqlx::PgPool, action_type: &str, user_id: i64) -> i32 {
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

fn get_current_monday_timestamp() -> i64 {
    let now = Utc::now();
    // Days since last Monday (Mon=0, Tue=1, ..., Sun=6 in ISO)
    let days_from_monday = now.weekday().num_days_from_monday();
    
    // Get to Monday 00:00:00
    let monday = (now - chrono::Duration::days(days_from_monday as i64))
        .date_naive()
        .and_time(NaiveTime::MIN); // 00:00:00
        
    Utc.from_local_datetime(&monday).unwrap().timestamp()
}

fn is_in_range(now: &str, starts_at: &str, ends_at: &str) -> bool {
    // Standard ISO 8601 strings (YYYY-MM-DD) are lexicographically comparable
    now >= starts_at && now <= ends_at
}

async fn get_sum_in_range(
    pool: &sqlx::PgPool,
    action_type: &str,
    user_id: i64,
    from_ts: i64,
    to_ts: i64
) -> i32 {
    // Convert Unix timestamps to Timestamptz for Postgres
    let start = Utc.timestamp_opt(from_ts, 0).unwrap();
    let end = Utc.timestamp_opt(to_ts, 0).unwrap();

    let result = sqlx::query!(
        r#"
        SELECT COUNT(*)::INT as count 
        FROM tasker_actions 
        WHERE action_type = $1 
            AND user_id = $2
            AND created_at >= $3 
            AND created_at <= $4
        "#,
        action_type,
        user_id,
        start,
        end
    )
    .fetch_one(pool)
    .await;

    if result.is_err() {
        println!("Database query error: {:?}", result.err());
        return 0;
    }
    let result = result.unwrap();

    result.count.unwrap_or(0)
}

async fn get_actions_by_type(pool: &sqlx::PgPool, action_type: &str, user_id: i64, day_str: &str) -> i32 {
    // "day_str" is "2026-02-13"
    // We query specifically for actions created on that UTC day

    // let day = NaiveTime::from_str(day_str).unwrap_or_else(|_| NaiveTime::from_hms_opt(0, 0, 0).unwrap()); 
    let day = NaiveDate::from_str(day_str)
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());
        // .and_hms_opt(0, 0, 0).unwrap();

    let result = sqlx::query!(
        r#"
        SELECT COUNT(*)::INT as count 
        FROM tasker_actions 
        WHERE action_type = $1 
            AND user_id = $2
            AND created_at::date = $3::date
        "#,
        action_type,
        user_id,
        day,
    )
    .fetch_one(pool)
    .await;

    if result.is_err() {
        println!("Database query error: {:?}", result.err());
        return 0;
    }
    let result = result.unwrap();
    result.count.unwrap_or(0)
}

async fn check_monthly_task(pool: &sqlx::PgPool, action_type: &str, user_id: i64) -> i32 {
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

async fn check_special_task(pool: &sqlx::PgPool, action_type: &str, user_id: i64) -> i32 {
    // For Special/Social/Partner tasks, we usually check the Lifetime Sum
    // We can use a range from Unix Epoch (0) to now.
    let to_ts = Utc::now().timestamp();
    
    get_sum_in_range(pool, action_type, user_id, 0, to_ts).await
}