use std::fmt::Debug;

use reqwest::StatusCode;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
// use strum_macros::{Display, EnumString, AsRefStr};

#[derive(Eq, Hash, PartialEq, Clone, Copy, Serialize, Deserialize)]
// #[derive(Eq, Hash, PartialEq, Clone, Copy, Serialize, Deserialize, Display, EnumString, AsRefStr, Debug)]
// #[strum(serialize_all = "UPPERCASE")]
pub enum Currency {
    PRESTIGE,
    STARS,
    TICKETS,
    TON,
    USDT
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

impl From<&str> for Currency {
    fn from(s: &str) -> Self {
        match s {
            "PRESTIGE" => Currency::PRESTIGE,
            "STARS" => Currency::STARS,
            "TICKETS" => Currency::TICKETS,
            "TON" => Currency::TON,
            "USDT" => Currency::USDT,
            _ => panic!("Unknown currency type: {}", s),
        }
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

impl From<u16> for Currency {
    fn from(value: u16) -> Self {
        match value {
            1 => Currency::PRESTIGE,
            2 => Currency::STARS,
            3 => Currency::TICKETS,
            4 => Currency::TON,
            5 => Currency::USDT,
            _ => panic!("Unknown currency type: {}", value),
        }
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

impl From<String> for Currency {
    fn from(value: String) -> Self {
        match value.to_uppercase().as_str() {
            "PRESTIGE" => Currency::PRESTIGE,
            "STARS" => Currency::STARS,
            "TICKETS" => Currency::TICKETS,
            "TON" => Currency::TON,
            "USDT" => Currency::USDT,
            _ => panic!("Unknown currency type: {}", value),
        }
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
        write!(f, "{}", currency_str)
    }
}

pub fn convert(price: Decimal, from: &Currency, to: &Currency, ton_price: Decimal) -> Decimal {
    if from == to {
        return price;
    }
    let from_course = get_course(from, ton_price);
    let to_course = get_course(to, ton_price);

    let result = (price * from_course) / to_course;

    // return Number(result.toFixed(2));
    // result.with_scale(2)
    result.normalize()
}

pub fn get_course(currency: &Currency, current_ton_usd: Decimal) -> Decimal {
    match currency {
        Currency::TON => current_ton_usd,
        Currency::PRESTIGE => dec!(0.03),
        Currency::STARS => dec!(0.02),
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

pub async fn add(
    currency: &Currency,
    amount: &Decimal,
    user_id: i64,
    client: &reqwest::Client,
) -> Result<(), (StatusCode, String)> {
    let prestige_url = "https://prestige.up.railway.app";

    let currency_id: u16 = (*currency).into();
    let external_url = format!(
        "{}/balance/add_currency/{}/{}/{}", 
        prestige_url, currency_id, amount, user_id
    );
    
    let resp = client
        .post(&external_url)
        // .header("Authorization", &state.internal_api_key)
        .send()
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "Balance service unreachable".to_string()))?;
    
    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        println!("External add_currency failed: {}", err);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("External add_currency failed: {}", err)));
    }

    Ok(())
}

pub async fn sub(
    currency: &Currency,
    amount: &Decimal,
    user_id: i64,
    client: &reqwest::Client,
) -> Result<(), (StatusCode, String)> {
    let prestige_url = "https://prestige.up.railway.app";

    let currency_id: u16 = (*currency).into();
    let external_url = format!(
        "{}/balance/sub_currency/{}/{}/{}", 
        prestige_url, currency_id, amount, user_id
    );
    
    let resp = client
        .post(&external_url)
        // .header("Authorization", &state.internal_api_key)
        .send()
        .await
        .map_err(|_| (StatusCode::BAD_GATEWAY, "Balance service unreachable".to_string()))?;
    
    if !resp.status().is_success() {
        let err = resp.text().await.unwrap_or_default();
        println!("External sub_currency failed: {}", err);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("External sub_currency failed: {}", err)));
    }

    Ok(())
}