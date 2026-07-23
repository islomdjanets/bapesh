use std::sync::OnceLock;
use crate::env;

static PROJECT: OnceLock<String> = OnceLock::new();

fn project_name() -> &'static str {
    PROJECT.get_or_init(|| env::get("PROJECT_NAME").expect("PROJECT_NAME IS NOT SET"))
}

pub enum EnergyOutcome {
    Spent(serde_json::Value),          // new state
    Grant(serde_json::Value),          // new state
    Refund(serde_json::Value),          // new state
    LimitReached(serde_json::Value),   // state with *_resets_at for the error UI
}

pub async fn spend(
    user_id: i64,
    amount: i32,
    action: &str,
    ref_id: &str,

    client: &reqwest::Client,
    internal_secret: &str,
) -> Result<EnergyOutcome, String> {
    let resp = client
        .post("https://prestige.up.railway.app/energy/spend")
        .header("X-Internal-Secret", internal_secret)
        .json(&serde_json::json!({
            "user_id": user_id, "amount": amount,
            "project": project_name(), "action": action, "ref_id": ref_id,
        }))
        .send().await.map_err(|e| format!("energy service unreachable: {e}"))?;

    let status = resp.status();
    let body: serde_json::Value = resp.json().await.unwrap_or_default();
    match status {
        reqwest::StatusCode::OK => Ok(EnergyOutcome::Spent(body)),
        reqwest::StatusCode::CONFLICT => Ok(EnergyOutcome::LimitReached(body)),
        s => Err(format!("energy spend failed ({s}): {body}")),
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
        .post("https://prestige.up.railway.app/energy/grant")
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
        .post("https://prestige.up.railway.app/energy/refund")
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
        reqwest::StatusCode::CONFLICT => Ok(EnergyOutcome::LimitReached(body)),
        s => Err(format!("energy refund failed ({s}): {body}")),
    }
}