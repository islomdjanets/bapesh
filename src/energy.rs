use std::sync::OnceLock;
use crate::env;

static PROJECT: OnceLock<String> = OnceLock::new();

fn project_name() -> &'static str {
    PROJECT.get_or_init(|| env::get("PROJECT_NAME").expect("PROJECT_NAME IS NOT SET"))
}

pub enum EnergyOutcome {
    Grant(serde_json::Value),          // new state
    Refund(serde_json::Value),          // new state
    LimitReached(serde_json::Value),   // state with *_resets_at for the error UI
}

/// Why a spend did not happen.
///
/// `spend` used to collapse every failure into one `String`, with a 409 — the
/// meter is empty — arriving as the serialized `EnergyState` and everything else
/// as prose. Nothing but the *shape* told them apart, so callers that needed the
/// difference had to parse the string and guess: gg_arcade carried a `classify`
/// function doing exactly that, for the sole purpose of getting back a value the
/// service had already sent structured.
///
/// **Deliberately not folded into `EnergyOutcome::LimitReached` the way `grant`
/// does it.** For a grant, hitting the cap is a normal outcome — something was
/// still granted, just less than asked. For a spend it is a failure: nothing was
/// charged and the work must not proceed. Moving it to the `Ok` side would make
/// `spend(...).map_err(...)?` — which is how raffle_bot buys a ticket — succeed
/// on an empty meter and hand out the goods for free, and because that caller
/// discards the value, no compiler would have said a word.
#[derive(Debug)]
pub enum EnergyError {
    /// 409. Carries the `EnergyState` blob, which is the point: the client draws
    /// a countdown from its `*_resets_at` fields, and a sentence cannot be
    /// counted down.
    LimitReached(serde_json::Value),
    /// Unreachable service, a 5xx, or a body that made no sense.
    Failed(String),
}

impl std::fmt::Display for EnergyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // The state blob is for the client to render, not for a log line.
            Self::LimitReached(_) => write!(f, "energy: the meter is empty"),
            Self::Failed(m) => write!(f, "energy: {m}"),
        }
    }
}

impl std::error::Error for EnergyError {}

pub async fn spend(
    user_id: i64,
    amount: i32,
    action: &str,
    ref_id: &str,

    client: &reqwest::Client,
    internal_secret: &str,
) -> Result<serde_json::Value, EnergyError> {
    let resp = client
        .post(format!("{}/energy/spend", crate::prestige::host()))
        .header("X-Internal-Secret", internal_secret)
        .json(&serde_json::json!({
            "user_id": user_id, "amount": amount,
            "project": project_name(), "action": action, "ref_id": ref_id,
        }))
        .send().await
        .map_err(|e| EnergyError::Failed(format!("service unreachable: {e}")))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    match status {
        // The `EnergyState` bare, as prestige sends it — this is what the
        // client redraws its meter from. No `EnergyOutcome` wrapper: a spend has
        // exactly one success shape, and an enum with one reachable variant only
        // ever bought callers a match arm they had to write and could not reach.
        reqwest::StatusCode::OK => Ok(body),
        reqwest::StatusCode::CONFLICT => Err(EnergyError::LimitReached(body)),
        s => Err(EnergyError::Failed(format!("spend failed ({s}): {body}"))),
    }
}

pub async fn grant(
    user_id: i64,
    amount: i32,
    action: &str,
    ref_id: &str,

    client: &reqwest::Client,
    internal_secret: &str,
) -> Result<EnergyOutcome, String> {
    let resp = client
        .post(format!("{}/energy/grant", crate::prestige::host()))
        .header("X-Internal-Secret", internal_secret)
        .json(&serde_json::json!({
            "user_id": user_id, "amount": amount,
            "project": project_name(), "action": action, "ref_id": ref_id,
        }))
        .send().await.map_err(|e| format!("energy service unreachable: {e}"))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    match status {
        reqwest::StatusCode::OK => Ok(EnergyOutcome::Grant(body)),
        reqwest::StatusCode::CONFLICT => Ok(EnergyOutcome::LimitReached(body)),
        s => Err(format!("energy grant failed ({s}): {body}")),
    }
}

pub async fn refund(
    ref_id: &str,

    client: &reqwest::Client,
    internal_secret: &str,
) -> Result<EnergyOutcome, String> {
    let resp = client
        .post(format!("{}/energy/refund", crate::prestige::host()))
        .header("X-Internal-Secret", internal_secret)
        .json(&serde_json::json!({
            "project": project_name(),
            "ref_id": ref_id,
        }))
        .send().await.map_err(|e| format!("energy service unreachable: {e}"))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    match status {
        reqwest::StatusCode::OK => Ok(EnergyOutcome::Refund(body)),
        // reqwest::StatusCode::CONFLICT => Err(EnergyOutcome::LimitReached(body)),
        s => Err(format!("energy refund failed ({s}): {body}")),
    }
}