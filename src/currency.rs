use core::panic;
use std::fmt::Debug;

use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};

#[derive(Eq, Hash, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum Currency {
    PRESTIGE,
    STARS,
    TICKETS,
    TON,
    USDT
}

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
            _ => panic!("Unknown currency type: {}", self),
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

pub fn convert(price: BigDecimal, from: &Currency, to: &Currency) -> BigDecimal {
    if from == to {
        return price;
    }
    let from_course = get_course(from);
    let to_course = get_course(to);

    let result = price * from_course / to_course;

    // return Number(result.toFixed(2));
    // result.with_scale(2)
    result
}

pub fn get_course(currency: &Currency) -> BigDecimal {
    // if TON_PRICE == BigDecimal::from(0) {
    //     println!("not initialised TON price");
    // }
    match currency {
        // Currency::TON => TON_PRICE,
        Currency::PRESTIGE => "0.03".parse().unwrap(),
        Currency::STARS => "0.02".parse().unwrap(),
        Currency::TICKETS => "0.1".parse().unwrap(),
        Currency::USDT => "1.0".parse().unwrap(),
        _ => panic!("Unknown currency type: {}", currency),
    }
}