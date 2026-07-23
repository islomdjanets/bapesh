use std::{collections::HashMap, fmt::Debug, sync::OnceLock};

use reqwest::StatusCode;
use rust_decimal::{Decimal, prelude::{FromPrimitive, ToPrimitive}};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use rust_decimal::RoundingStrategy;

use crate::env;
// use strum_macros::{Display, EnumString, AsRefStr};

#[derive(Eq, Hash, PartialEq, Clone, Copy, Serialize, Deserialize)]
// #[derive(Eq, Hash, PartialEq, Clone, Copy, Serialize, Deserialize, Display, EnumString, AsRefStr, Debug)]
// #[strum(serialize_all = "UPPERCASE")]
pub enum Currency {
    PRESTIGE = 1,
    STARS = 2,
    TICKETS = 3,
    TON = 4,
    USDT = 5,
}

// impl From<Currency> for u16 {
//     fn from(value: Currency) -> u16 {
//         match value {
//             Currency::PRESTIGE => 1,
//             Currency::STARS => 2,
//             Currency::TICKETS => 3,
//             Currency::TON => 4,
//             Currency::USDT => 5,
//         }
//     }
// }

impl Debug for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Currency::STARS => write!(f, "STARS"),
            Currency::TON => write!(f, "TON"),
            Currency::USDT => write!(f, "USDT"),
            Currency::PRESTIGE => write!(f, "PRESTIGE"),
            Currency::TICKETS => write!(f, "TICKETS"),
        }
    }
}

impl TryFrom<&str> for Currency {
    type Error = ();
    fn try_from(s: &str) -> Result<Self, ()> {
        Ok(match s {
            "PRESTIGE" => Currency::PRESTIGE,
            "STARS" => Currency::STARS,
            "TICKETS" => Currency::TICKETS,
            "TON" => Currency::TON,
            "USDT" => Currency::USDT,
            _ => return Err(()),
        })
    }
}

impl AsRef<str> for Currency {
    fn as_ref(&self) -> &str {
        match self {
            Currency::PRESTIGE => "PRESTIGE",
            Currency::STARS => "STARS",
            Currency::TICKETS => "TICKETS",
            Currency::TON => "TON",
            Currency::USDT => "USDT",
        }
    }
}

impl TryFrom<u16> for Currency {
    type Error = ();
    fn try_from(value: u16) -> Result<Self, ()> {
        Ok(match value {
            1 => Currency::PRESTIGE,
            2 => Currency::STARS,
            3 => Currency::TICKETS,
            4 => Currency::TON,
            5 => Currency::USDT,
            _ => return Err(()),
        })
    }
}

impl Into<u16> for Currency {
    fn into(self) -> u16 {
        match self {
            Currency::PRESTIGE => 1,
            Currency::STARS => 2,
            Currency::TICKETS => 3,
            Currency::TON => 4,
            Currency::USDT => 5,
        }
    }
}

impl TryFrom<String> for Currency {
    type Error = ();
    fn try_from(value: String) -> Result<Self, ()> {
        Ok(match value.to_uppercase().as_str() {
            "PRESTIGE" => Currency::PRESTIGE,
            "STARS" => Currency::STARS,
            "TICKETS" => Currency::TICKETS,
            "TON" => Currency::TON,
            "USDT" => Currency::USDT,
            _ => return Err(()),
        })
    }
}

impl std::fmt::Display for Currency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let currency_str = match self {
            Currency::PRESTIGE => "PRESTIGE",
            Currency::STARS => "STARS",
            Currency::TICKETS => "TICKETS",
            Currency::TON => "TON",
            Currency::USDT => "USDT",
        };

        return write!(f, "{}", currency_str);
    }
}

pub fn parse(id: i16) -> Result<Currency, (StatusCode, String)> {
    u16::try_from(id).ok()
        .and_then(|v| Currency::try_from(v).ok())
        .ok_or((StatusCode::BAD_REQUEST, "Unknown currency".into()))
}


pub enum RoundMode {
    Nearest, // Standard rounding: .5 up, .4 down
    Down,    // Floor: Always rounds towards zero
    Up,      // Ceil: Always rounds away from zero
}

pub fn convert(
    price: Decimal,
    from: &Currency,
    to: &Currency,
    ton_price: Decimal,
    round_mode: RoundMode,
) -> Decimal {
    if from == to {
        return price;
    }
    let from_course = get_course(from, ton_price);
    let to_course = get_course(to, ton_price);

    let mut result = (price * from_course) / to_course;

    // return Number(result.toFixed(2));
    // result.with_scale(2)
    // result.normalize()
    // to_units(result, to)

    // Map your custom RoundMode to rust_decimal::RoundingStrategy
    let strategy = match round_mode {
        RoundMode::Nearest => RoundingStrategy::MidpointAwayFromZero,
        RoundMode::Down => RoundingStrategy::ToZero,
        RoundMode::Up => RoundingStrategy::AwayFromZero,
    };

    result = match to {
        Currency::STARS => result.round_dp_with_strategy(0, strategy),
        Currency::TON => result.round_dp_with_strategy(9, strategy),
        Currency::PRESTIGE | Currency::USDT => result.round_dp_with_strategy(2, strategy),
        _ => result.normalize(),
    };

    result
}

pub fn to_units(amount: Decimal, currency: &Currency) -> i64 {
    match currency {
        // TON: 9 decimals. 1.0 TON -> 1,000,000,000 nanotons
        Currency::TON => (amount * dec!(1_000_000_000)).to_i64().unwrap_or(0),
        
        // STARS: 0 decimals. 50.0 STARS -> 50
        Currency::STARS => amount.to_i64().unwrap_or(0),
        
        // PRESTIGE: 2 decimals. 10.50 -> 1050 (if your service uses cents)
        Currency::PRESTIGE => (amount * dec!(100)).to_i64().unwrap_or(0),
        
        _ => amount.to_i64().unwrap_or(0),
    }
}

pub fn get_course(currency: &Currency, current_ton_usd: Decimal) -> Decimal {
    match currency {
        Currency::TON => current_ton_usd,
        Currency::PRESTIGE => dec!(0.03),
        Currency::STARS => dec!(0.015),
        Currency::TICKETS => dec!(0.1),
        Currency::USDT => dec!(1.0),
    }
}

// pub fn from_nanoton(ton: u64) -> Decimal {
//     let nanoton_in_decimal: Decimal = Decimal::new(1, 9); // 1 TON = 10^9 nanoton
//     Decimal::from(ton) / nanoton_in_decimal
// }

pub fn from_nanoton(nanoton: u64) -> Decimal {
    // This takes the integer and moves the decimal point 9 places to the left.
    // e.g., 1_000_000_000 becomes 1.000000000
    Decimal::from_i128_with_scale(nanoton as i128, 9)
}

pub async fn add_multiple(
    currencies: &HashMap<Currency, Decimal>,
    user_id: i64,
    client: &reqwest::Client,
    internal_secret: &String,
) -> Result<(), (StatusCode, String)> {
    let prestige_url = "https://prestige.up.railway.app";

    // 1. Keep the URL clean (only ID in path)
    let external_url = format!("{}/balance/add_currencies/{}", prestige_url, user_id);
    
    // let external_url = format!(
    //     "{}/balance/add_currencies/{}/{}", 
    //     prestige_url, serde_json::to_string(currencies).unwrap(), user_id
    // );
    
    let resp = client
        .post(&external_url)
        .header("X-Internal-Secret", internal_secret)
        .json(currencies)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Network Error: {:?}", e);
            (StatusCode::BAD_GATEWAY, "Balance service unreachable".to_string())
        })?;
    
    if !resp.status().is_success() {
        let err_status = resp.status();
        let err_body = resp.text().await.unwrap_or_else(|_| "No error body".to_string());
        
        let message = format!("Balance service error ({}): {}", err_status, err_body);
        eprintln!("{}", message);

        return Err((StatusCode::INTERNAL_SERVER_ERROR, message));
    }

    Ok(())
}

pub async fn add(
    currency: &Currency,
    amount: &Decimal,
    user_id: i64,

    client: &reqwest::Client,
    internal_secret: &String,
    is_deposit: bool
) -> Result<(), (StatusCode, String)> {
    let prestige_url = "https://prestige.up.railway.app";

    let currency_id: u16 = (*currency).into();
    let external_url = format!(
        "{}/balance/add_currency/{}/{}/{}",
        prestige_url, currency_id, amount, user_id
    );

    // let tasker_host = env::get("TASKER_HOST").unwrap_or_default();
    
    let resp = client
        .post(&external_url)
        .header("X-Internal-Secret", internal_secret)
        .header("Is-Deposit", if is_deposit {"true"} else {"false"})
        // .header("Tasker-Host", tasker_host)
        .send()
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "Balance service unreachable".to_string()))?;
    
    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        let message = format!("External add_currency failed: {}", err);
        println!("{}", message);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, message));
    }

    Ok(())
}

pub async fn sub(
    currency: &Currency,
    amount: &Decimal,
    user_id: i64,

    client: &reqwest::Client,
    internal_secret: &String,
) -> Result<(), (StatusCode, String)> {
    let prestige_url = "https://prestige.up.railway.app";

    let currency_id: u16 = (*currency).into();
    let external_url = format!(
        "{}/balance/sub_currency/{}/{}/{}", 
        prestige_url, currency_id, amount, user_id
    );
    
    // let tasker_host = env::get("TASKER_HOST").unwrap_or_default();

    let resp = client
        .post(&external_url)
        .header("X-Internal-Secret", internal_secret)
        // .header("Tasker-Host", tasker_host)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Network Error: {:?}", e);
            (StatusCode::BAD_GATEWAY, "Balance service unreachable".to_string())
        })?;
    
    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        let message = format!("External sub_currency failed: {}", err);
        println!("{}", message);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, message));
    }

    Ok(())
}

pub fn f32_to_decimal(value: f32) -> Option<Decimal> {
    Decimal::from_f32_retain(value)
}

pub fn f64_to_decimal(value: f64) -> Option<Decimal> {
    Decimal::from_f64_retain(value)
}

pub fn i32_to_decimal(value: i32) -> Option<Decimal> {
    Decimal::from_i32(value)
}

pub fn i64_to_decimal(value: i64) -> Option<Decimal> {
    Decimal::from_i64(value)
}

pub async fn transfer(
    currency: &Currency,
    amount: &Decimal,

    user_id: i64,
    receiver_id: i64,
    fee: &Decimal,

    client: &reqwest::Client,
    internal_secret: &String,
) -> Result<String, (StatusCode, String)> {
    let prestige_url = "https://prestige.up.railway.app";

    let currency_id: u16 = (*currency).into();
    let external_url = format!(
        "{}/balance/transfer/{}/{}/{}/{}/{}", 
        prestige_url, currency_id, amount, user_id, receiver_id, fee,
    );
    
    // let tasker_host = env::get("TASKER_HOST").unwrap_or_default();

    let resp = client
        .post(&external_url)
        .header("X-Internal-Secret", internal_secret)
        // .header("Tasker-Host", tasker_host)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Network Error: {:?}", e);
            (StatusCode::BAD_GATEWAY, "Balance service unreachable".to_string())
        })?;
    
    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        let message = format!("External transfer failed: {}", err);
        println!("{}", message);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, message));
    }

    let result = resp.text().await;
    match result {
        Ok(text) => Ok(text),
        Err(e) => {
            let message = format!("Failed to read transfer response: {:?}", e);
            println!("{}", message);
            Err((StatusCode::INTERNAL_SERVER_ERROR, message))
        }
    }
}

static PROJECT: OnceLock<String> = OnceLock::new();

fn project_name() -> &'static str {
    PROJECT.get_or_init(|| env::get("PROJECT_NAME").expect("PROJECT_NAME IS NOT SET"))
}

// e.g. in the bapesh crate next to currency::add
pub enum EnergyOutcome {
    Spent(serde_json::Value),          // new state
    Grant(serde_json::Value),          // new state
    LimitReached(serde_json::Value),   // state with *_resets_at for the error UI
}

pub async fn spend_energy(
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

pub async fn grant_energy(
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