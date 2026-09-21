// use urlencoding::decode;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Sha256};

use std::{eprintln, error::Error};
use serde::{Serialize, Deserialize};

use crate::env;

// #[derive(Debug, Serialize, Deserialize)]
// pub struct User {
//     pub id: i64,
//     first_name: String,
//     last_name: String,
//     username: String,
//     language_code: String,
//     is_premium: bool,
//     allows_write_to_pm: bool,
//     pub photo_url: String,
// }

// async fn start(message: Message, bot: Bot) -> ResponseResult<()> {
//     let mut params = Vec::new();
//     if let Some(text) = message.text() {
//         let args: Vec<&str> = text.splitn(2, ' ').collect();
//         let data_str = if args.len() > 1 { args[1] } else { "" };

//         let decoded_data = URL_SAFE
//             .decode(data_str)
//             .ok()
//             .and_then(|bytes| String::from_utf8(bytes).ok());

//         if let Some(decoded_data) = decoded_data {
//             let ref_index = decoded_data.find("r=");
//             let query_index = decoded_data.find("q=");
//             if let Some(ref_index) = ref_index {
//                 let referral_id =
//                     &decoded_data[ref_index + 2..query_index.unwrap_or(decoded_data.len())];
//                 params.push(format!("ref={}", referral_id));
//             }
//             if let Some(query_index) = query_index {
//                 let query_id = &decoded_data[query_index + 2..];
//                 params.push(format!("q={}", query_id));
//             }
//         }
//     }

//     let premium_user_status = message.from().map_or(false, |user| user.is_premium);
//     if premium_user_status {
//         params.push(format!("pr={}", premium_user_status));
//     }

//     let url = if params.is_empty() {
//         URL.to_string()
//     } else {
//         format!("{}?{}", URL, params.join("&"))
//     };

//     // Convert the URL string to a reqwest::Url
//     let url = reqwest::Url::parse(&url).expect("Invalid URL");

//     // let inline_kb = InlineKeyboardMarkup::new(
//     //     vec![vec![InlineKeyboardButton::url(
//     //         "Open the App",
//     //         url,
//     //     )]]
//     // );

//     let mini_app_button = InlineKeyboardButton::new(
//         "Open the App",
//         WebApp( WebAppInfo {
//             url,
//         })
//     );

//     let keyboard = InlineKeyboardMarkup::new(vec![vec![mini_app_button]]);

//     bot
//         .send_message(
//             message.chat.id,
//             // format!("Hello! This is a test bot. You can visit the web page by clicking the button below.\n\n{}\n<a href='{}'>URL</a>", url, url)
//             "Welcome to Rogarlic Beta! Collect resources, improve production and collect all of the NFT's to take advantage in actual game and MAX PROFIT"
//         )
//         .parse_mode(ParseMode::Html)
//         // .reply_markup(inline_kb).await?;
//         .reply_markup(keyboard).await?;

//     Ok(())
// }

// pub struct Telegram {

// }

// impl Service for Telegram {
//     fn uri(&self, request: &crate::handshake::Request) -> bool {
//         todo!()
//     }

//     fn handler(&self, stream: &mut tokio::net::TcpStream, resources: std::sync::Arc<std::sync::Mutex<crate::server::Resources>>) {
//         todo!()
//     }

// }

// pub async fn serve() {
//     let token = env::get("TOKEN").expect("TOKEN not set");

//     let bot = Bot::new(token);

//     teloxide::repl(bot.clone(), move |message| {
//         let bot = bot.clone();
//         async move {
//             start(message, bot).await.log_on_error().await;
//             respond(())
//         }
//     }).await;
// }

// pub async fn handle_connection( stream: &mut TcpStream, _: Arc<Mutex<Resources>> ) {
//     // let token = env::var("TOKEN").expect("TOKEN not set");
//     let token = env::get("TOKEN").expect("TOKEN not set");
//     println!("{}", token);
//
//     // Initialize the bot with the token
//     let bot = Bot::new(token);
//
//     teloxide::repl(bot.clone(), move |message| {
//         let bot = bot.clone();
//         async move {
//             start(message, bot).await.log_on_error().await;
//             respond(())
//         }
//     }).await;
// }

// #[tokio::main]
// async fn main() {
//     let bot = Bot::from_env().auto_send();
//
//     teloxide::repl(bot, |message: Message| async move {
//         if let Some(text) = message.text() {
//             if text == "/start" {
//                 let url = reqwest::Url::parse(&URL.to_string()).expect("Invalid URL");
//
//                 let mini_app_button = InlineKeyboardButton::new(
//                     "Open the App",
//                     teloxide::types::InlineKeyboardButtonKind::WebApp( teloxide::types::WebAppInfo {
//                         url,
//                     })
//                 );
//
//                 let keyboard = InlineKeyboardMarkup::new(vec![vec![mini_app_button]]);
//
//                 bot
//                     .send_message(
//                         message.chat.id,
//                         // format!("Hello! This is a test bot. You can visit the web page by clicking the button below.\n\n{}\n<a href='{}'>URL</a>", url, url)
//                         "Welcome to Rogarlic Beta! Collect resources, improve production and collect all of the NFT's to take advantage in actual game and MAX PROFIT"
//                     )
//                     .parse_mode(ParseMode::Html)
//                     // .reply_markup(inline_kb).await?;
//                     .reply_markup(keyboard).await?;
//             }
//         }
//
//         respond(())
//     })
//     .await;
// }

// pub fn extract_user(init_data: &str) -> Option<User> {
//     let params: HashMap<String, String> = init_data
//         .split('&')
//         .filter_map(|pair| {
//             let mut split = pair.splitn(2, '=');
//             Some((split.next()?, split.next()?))
//         })
//         .map(|(k, v)| (k.to_string(), v.to_string()))
//         .collect();

//     let user_encoded = params.get("user")?;
//     // println!("Encoded user data: {}", user_encoded);
//     let decoded = decode(user_encoded).ok()?;
//     // println!("Decoded user data: {}", decoded);

//     // if is_premium is missing, add it with false value
//     let mut decoded: String = decoded.into_owned();
//     if !decoded.contains("\"is_premium\"") {
//         decoded = decoded.replace("}", ",\"is_premium\":false}");
//         // println!("Decoded user data with is_premium: {}", decoded);
//     }


//     let user = serde_json::from_str::<User>(&decoded);
//     if user.is_err() {
//         println!("Error parsing user JSON: {:?}", user.err());
//         return None;
//     }
//     let user = user.unwrap();
//     // println!("Extracted user: {:?}", user);
//     Some(user)
// }

// pub fn validate_init_data(init_data: &str, bot_token: &str) -> Result<bool, Box<dyn Error>> {
//     // Parse the initData query string
//     // println!("Raw init_data: {}", init_data);
//     let init_data = decode(init_data)?.into_owned();
//     // println!("Decoded init_data: {}", init_data);

//     let mut pairs: Vec<&str> = init_data.split('&').collect();
    
//     // Extract the hash
//     let hash = pairs.iter()
//         .find(|&&pair| pair.starts_with("hash="))
//         .ok_or("Hash not found in initData")?
//         .strip_prefix("hash=")
//         .ok_or("Invalid hash format")?;
    
//     // Remove the hash from pairs and sort the remaining key-value pairs
//     pairs.retain(|&pair| !pair.starts_with("hash="));
//     pairs.sort();
    
//     // Join the pairs with newline to create data_check_string
//     let data_check_string = pairs.join("\n");
    
//     // Create the secret key: HMAC-SHA256("WebAppData", bot_token)
//     let mut secret_hmac = Hmac::<Sha256>::new_from_slice("WebAppData".as_bytes())?;
//     secret_hmac.update(bot_token.as_bytes());
//     let secret_key = secret_hmac.finalize().into_bytes();
    
//     // Compute HMAC-SHA256(data_check_string, secret_key)
//     let mut hmac = Hmac::<Sha256>::new_from_slice(&secret_key)?;
//     hmac.update(data_check_string.as_bytes());
//     let computed_hash = hex::encode(hmac.finalize().into_bytes());
    
//     // Compare computed hash with provided hash
//     // println!("Data check string: {}", data_check_string);
//     // println!("Computed hash: {}", computed_hash);
//     // println!("Provided hash: {}", hash);

//     Ok(computed_hash == hash)
// }


// GEMINI
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub first_name: String,
    #[serde(default)] // If missing, defaults to empty string
    pub last_name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub language_code: String,
    #[serde(default)] // If missing, defaults to false
    pub is_premium: bool,
    #[serde(default)]
    pub allows_write_to_pm: bool,
    #[serde(default)]
    pub photo_url: String,
}

pub fn validate_init_data(raw_init_data: &str, bot_token: &str) -> Result<bool, Box<dyn Error>> {
    // 1. Parse query string into key-value pairs
    let mut params: Vec<(String, String)> = Vec::new();
    let mut provided_hash = String::new();

    for pair in raw_init_data.split('&') {
        let mut split = pair.splitn(2, '=');
        let key = split.next().ok_or("Invalid pair")?;
        let value = split.next().ok_or("Invalid value")?;
        
        // URL decode the value only
        let decoded_value = urlencoding::decode(value)?;

        if key == "hash" {
            provided_hash = decoded_value.into_owned();
        }
        //  else if key == "signature" {
        //     // SKIP the signature field for HMAC validation
        //     continue;
        // }
         else {
            params.push((key.to_string(), decoded_value.into_owned()));
        }
    }

    if provided_hash.is_empty() {
        return Err("No hash found".into());
    }
    
    const MAX_INIT_DATA_AGE_SECS: i64 = 3600; // 1 hour is plenty for a mini-app launch

    // let auth_date: i64 = params.iter()
    //     .find(|(k, _)| k == "auth_date")
    //     .and_then(|(_, v)| v.parse().ok())
    //     .ok_or("Missing auth_date")?;

    // let age = chrono::Utc::now().timestamp() - auth_date;
    // if age > MAX_INIT_DATA_AGE_SECS || age < -300 {
    //     eprintln!("auth_data is wrong");
    //     return Ok(false); // stale (replayed) or clock-skewed init data
    // }

    // let auth_date = params.iter().find(|(k, _)| k == "auth_date")...
    // if current_time - auth_date > 86400 { return Ok(false); }

    // 2. Sort parameters alphabetically
    params.sort_by(|a, b| a.0.cmp(&b.0));

    // 3. Construct data_check_string
    let data_check_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    // 4. Generate Secret Key
    let mut secret_hmac = Hmac::<Sha256>::new_from_slice(b"WebAppData")?;
    secret_hmac.update(bot_token.as_bytes());
    let secret_key = secret_hmac.finalize().into_bytes();

    // 5. Compute Hash
    let mut hmac = Hmac::<Sha256>::new_from_slice(&secret_key)?;
    hmac.update(data_check_string.as_bytes());
    
    // Convert provided hex hash to bytes for verification
    let provided_hash_bytes = hex::decode(&provided_hash)?;
    
    // .verify_slice() performs a constant-time comparison
    Ok(hmac.verify_slice(&provided_hash_bytes).is_ok())
}

pub fn extract_user(init_data: &str) -> Option<User> {
    // We can use a simple split here since init_data is already validated
    let user_param = init_data
        .split('&')
        .find(|pair| pair.starts_with("user="))?
        .strip_prefix("user=")?;

    let decoded_json = urlencoding::decode(user_param).ok()?;
    serde_json::from_str::<User>(&decoded_json).ok()
}

// ── the other two ways Telegram signs a login ──────────────────────────────
//
// `validate_init_data` above is the bot's own check: HMAC keyed by the bot
// token, which only that bot's server holds. An identity service that signs
// people in for *every* bot in the ecosystem needs two more:
//
// - **Third-party initData validation.** Since Bot API 7.10 initData carries a
//   `signature` — Ed25519 over `{bot_id}:WebAppData\n{fields}` — that anyone
//   can verify against Telegram's published public key, with no bot token at
//   all. The bot id has to be known, which is what makes it safe to accept: a
//   service verifies only for the bots it has been told about, so initData
//   minted for some stranger's bot can never log into an account here.
// - **The Login Widget** (`oauth.telegram.org`), which is how a person signs
//   in with Telegram from an ordinary browser. It signs the same way as a bot
//   webhook rather than as a Mini App: HMAC-SHA256 keyed by `SHA256(token)`,
//   not by `HMAC("WebAppData", token)`. One byte of difference in the key
//   derivation, and a validator that gets it wrong fails every login with
//   nothing to distinguish it from a forgery.

/// Telegram's Ed25519 public key for the production environment, from the
/// Bot API documentation on validating data for third-party use.
pub const PUBLIC_KEY: [u8; 32] = [
    0xe7, 0xbf, 0x03, 0xa2, 0xfa, 0x46, 0x02, 0xaf, 0x45, 0x80, 0x70, 0x3d, 0x88, 0xdd, 0xa5, 0xbb,
    0x59, 0xf3, 0x2e, 0xd8, 0xb0, 0x2a, 0x56, 0xc1, 0x87, 0xfe, 0x7d, 0x34, 0xca, 0xed, 0x24, 0x2d,
];

/// The `key=value` pairs of an initData (or Login Widget) query string, URL
/// decoded, in the order they arrived. `hash` and `signature` come back like
/// any other field; the validators know which to leave out of the check string.
pub fn parse_init_data(raw: &str) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut params = Vec::new();

    for pair in raw.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut split = pair.splitn(2, '=');
        let key = split.next().ok_or("Invalid pair")?;
        let value = split.next().ok_or("Invalid value")?;

        params.push((key.to_string(), urlencoding::decode(value)?.into_owned()));
    }

    Ok(params)
}

/// The data-check-string every Telegram signature is computed over: the
/// remaining fields sorted by key, `key=value`, joined by newlines.
fn check_string(params: &[(String, String)], skip: &[&str]) -> String {
    let mut fields: Vec<&(String, String)> = params
        .iter()
        .filter(|(k, _)| !skip.contains(&k.as_str()))
        .collect();
    fields.sort_by(|a, b| a.0.cmp(&b.0));

    fields
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
}

fn field<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// `auth_date` of a signed payload, as a unix timestamp.
pub fn auth_date(params: &[(String, String)]) -> Option<i64> {
    field(params, "auth_date")?.parse().ok()
}

/// Verifies initData against Telegram's public key, for the bot it names.
///
/// `Ok(false)` is a signature that does not verify — for this bot id, or at
/// all. `Err` is initData with no `signature` field, which a client older than
/// Bot API 7.10 still sends; the caller decides whether the HMAC path with a
/// bot token is available instead.
pub fn validate_init_data_signature(raw: &str, bot_id: i64) -> Result<bool, Box<dyn Error>> {
    let params = parse_init_data(raw)?;
    let signature = field(&params, "signature").ok_or("No signature found")?;

    verify_signature(&params, signature, bot_id, &PUBLIC_KEY)
}

/// The check behind `validate_init_data_signature`, with the key as an
/// argument so a test can sign with a key it holds.
pub fn verify_signature(
    params: &[(String, String)],
    signature: &str,
    bot_id: i64,
    public_key: &[u8; 32],
) -> Result<bool, Box<dyn Error>> {
    use base64::Engine;

    let check = format!("{}:WebAppData\n{}", bot_id, check_string(params, &["hash", "signature"]));

    // Documented as base64url; tolerate the padding some encoders add.
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(signature.trim_end_matches('='))?;

    let key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, public_key);
    Ok(key.verify(check.as_bytes(), &signature).is_ok())
}

/// Verifies the fields the Login Widget hands to its callback URL
/// (`id`, `first_name`, `auth_date`, `hash`, ...), for the bot whose token
/// this is. The widget only ever signs for the one bot whose domain the page
/// is on, so unlike initData there is no bot id to name.
pub fn validate_login_widget(params: &[(String, String)], bot_token: &str) -> Result<bool, Box<dyn Error>> {
    use sha2::Digest;

    let provided = field(params, "hash").ok_or("No hash found")?;
    let check = check_string(params, &["hash"]);

    let secret_key = Sha256::digest(bot_token.as_bytes());

    let mut hmac = Hmac::<Sha256>::new_from_slice(&secret_key)?;
    hmac.update(check.as_bytes());

    Ok(hmac.verify_slice(&hex::decode(provided)?).is_ok())
}

#[cfg(test)]
mod signing_tests {
    use super::*;

    // A Login Widget payload signed by hand with node's crypto for the token
    // below — the vector is the contract, not the code that recomputes it.
    const TOKEN: &str = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
    const WIDGET: &str = "id=863009768&first_name=Islom&username=islomdjanets&auth_date=1758240000&hash=";

    fn widget(hash: &str) -> Vec<(String, String)> {
        parse_init_data(&format!("{}{}", WIDGET, hash)).unwrap()
    }

    const WIDGET_HASH: &str = "77210ccc9bdb6f5d5bbecd3eaf69b932ee6ab16c33aa432b1ad68c4370130ed3";

    #[test]
    fn widget_hash_is_hmac_keyed_by_sha256_of_the_token() {
        assert_eq!(validate_login_widget(&widget(WIDGET_HASH), TOKEN).unwrap(), true);
    }

    #[test]
    fn widget_rejects_a_tampered_field_and_the_wrong_key_derivation() {
        let mut forged = widget(WIDGET_HASH);
        forged[0].1 = "1".into();
        assert_eq!(validate_login_widget(&forged, TOKEN).unwrap(), false);

        // The Mini App derivation (`HMAC("WebAppData", token)`) over the same
        // fields is a different hash, so a widget payload must not pass the
        // initData validator and vice versa.
        let as_init_data = format!("{}{}", WIDGET, WIDGET_HASH);
        assert_eq!(validate_init_data(&as_init_data, TOKEN).unwrap(), false);
    }

    // initData for bot 42, signed with a throwaway Ed25519 key whose public
    // half is below. The production key is the constant; the check is the same.
    const TEST_KEY: [u8; 32] = [
        0xce, 0xd7, 0x56, 0x7a, 0xb3, 0xfa, 0x44, 0x1d, 0x5e, 0x61, 0x8f, 0x08, 0x2e, 0xf0, 0x97, 0x17,
        0x6c, 0x13, 0xa6, 0x7a, 0x63, 0x97, 0xb3, 0x24, 0x4f, 0x7e, 0xdc, 0x99, 0xb7, 0xb8, 0x56, 0x8e,
    ];
    const INIT: &str = "auth_date=1758240000&user=%7B%22id%22%3A863009768%2C%22first_name%22%3A%22Islom%22%7D&query_id=AAH&hash=00&signature=eD-Zj8WScx6OU9fAsaMmtXCdqnqMnTBDsAdyLrUX19zqgF0XkUdBlM9-xlo5qdrFuM6pZxuW2sWdVKR0mt6yDg";

    fn signed() -> (Vec<(String, String)>, String) {
        let params = parse_init_data(INIT).unwrap();
        let signature = field(&params, "signature").unwrap().to_string();
        (params, signature)
    }

    #[test]
    fn signature_verifies_for_the_bot_it_was_issued_to() {
        let (params, signature) = signed();
        assert_eq!(verify_signature(&params, &signature, 42, &TEST_KEY).unwrap(), true);
        // Padding is tolerated.
        assert_eq!(verify_signature(&params, &format!("{}==", signature), 42, &TEST_KEY).unwrap(), true);
    }

    #[test]
    fn signature_fails_for_another_bot_or_a_tampered_field() {
        let (params, signature) = signed();
        // The bot id is part of the signed string: initData minted for one bot
        // is worthless to a service that only knows another.
        assert_eq!(verify_signature(&params, &signature, 43, &TEST_KEY).unwrap(), false);

        let mut forged = params.clone();
        forged[0].1 = "1758240001".into();
        assert_eq!(verify_signature(&forged, &signature, 42, &TEST_KEY).unwrap(), false);

        // Against the real key it is simply not Telegram's signature.
        assert_eq!(validate_init_data_signature(INIT, 42).unwrap(), false);
    }

    #[test]
    fn widget_without_a_hash_is_an_error_not_a_pass() {
        let params = parse_init_data("id=1&auth_date=1").unwrap();
        assert!(validate_login_widget(&params, TOKEN).is_err());
    }

    #[test]
    fn init_data_without_a_signature_is_an_error_not_a_pass() {
        assert!(validate_init_data_signature("user=%7B%7D&auth_date=1&hash=00", 1).is_err());
    }

    #[test]
    fn check_string_sorts_and_skips() {
        let params = parse_init_data("b=2&signature=s&a=1&hash=h").unwrap();
        assert_eq!(check_string(&params, &["hash", "signature"]), "a=1\nb=2");
        assert_eq!(check_string(&params, &["hash"]), "a=1\nb=2\nsignature=s");
    }

    #[test]
    fn parse_decodes_values_and_keeps_order() {
        let params = parse_init_data("user=%7B%22id%22%3A1%7D&auth_date=5").unwrap();
        assert_eq!(params[0], ("user".into(), "{\"id\":1}".into()));
        assert_eq!(auth_date(&params), Some(5));
    }
}

#[derive(serde::Serialize)]
struct TelegramMessage {
    chat_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    message_thread_id: Option<i32>,

    text: String,
    parse_mode: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    disable_web_page_preview: Option<bool>,
}

pub async fn post(
    post: String,
    chat_id: String,
    thread_id: Option<i32>,
    bot_token: &str,

    client: &Client,
) -> Result<(), reqwest::Error> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    // let thread_id = match mode {
    //     Mode::Survival => 2, // Survival thread ID
    //     Mode::FCFS => 14,     // FCFS thread ID
    // };

    let payload = TelegramMessage {
        chat_id: chat_id,
        message_thread_id: thread_id, // The "Winners List" thread
        text: post,
        parse_mode: "HTML".to_string(),
        disable_web_page_preview: Some(true), // Keeps the list clean if seeds look like links
    };

    let response = client
        .post(url)
        .json(&payload)
        .send()
        .await?;

    // 3. Robust Error Logging
    if !response.status().is_success() {
        let status = response.status();
        let err_body = response.text().await.unwrap_or_default();
        eprintln!("Telegram API Error (Status {}): {}", status, err_body);
        return Ok(());
    }

    // println!("Successfully posted winners to Telegram Topic {}", thread_id);
    Ok(())
}

pub async fn notify(
    user_id: i64, 
    text: &str,
    bot_token: &str,

    client: &Client,
) -> Result<(), reqwest::Error> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

    let payload = serde_json::json!({
        "chat_id": user_id, // In your app, user_id is usually the Telegram chat_id
        "text": text,
        "parse_mode": "HTML"
    });

    client
        .post(url)
        .json(&payload)
        .send()
        .await?;

    Ok(())
}

// pub async fn profile_picture(
//     client: &Client,
//     Query(params): Query<ProfilePictureParams>,
// ) -> Result<Response, AppError> {
//     let bot_token = env::get("TOKEN")
//         .expect("TOKEN IS NOT SET");

//     let user_id = params.user_id;

//     let url = format!(
//         "https://api.telegram.org/bot{}/getUserProfilePhotos?user_id={}",
//         bot_token, user_id
//     );
//     let response = client
//         .get(&url)
//         .send()
//         .await?;
//         // .map_err(AppError::from);

//     if response.is_err() {
//         println!("Error fetching user profile photos: {:?}", response.as_ref().err());
//         return Err(AppError::from(response.err().unwrap()));
//     }

//     let response = response
//         .unwrap()
//         .json()
//         .await
//         .map_err(AppError::from);

//     if response.is_err() {
//         println!("Error parsing user profile photos response: {:?}", response.as_ref().err());
//         return Err(AppError::from(response.err().unwrap()));
//     }

//     let response: TelegramResponse<UserProfilePhotos> = response.unwrap();

//     if !response.ok || response.result.is_none() || response.result.as_ref().unwrap().photos.is_empty() {
//         // println!("No profile photos found for user_id: {}", user_id);
//         return Err(AppError::NotFound("No profile picture found".to_string()));
//     }

//     let file_id = response.result.unwrap().photos[0].last().unwrap().file_id.clone();

//     let url = format!(
//         "https://api.telegram.org/bot{}/getFile?file_id={}",
//         bot_token, &file_id
//     );
//     let response: TelegramResponse<FileResponse> = state.client
//         .get(&url)
//         .send()
//         .await
//         .map_err(AppError::from)?
//         .json()
//         .await
//         .map_err(AppError::from)?;

//     if !response.ok || response.result.is_none() {
//         println!("Failed to get file path for file_id: {}", file_id);
//         return Err(AppError::Internal("Failed to get file path".to_string()));
//     }

//     let file_path = response.result.unwrap().file_path;

//     let file_url = format!("https://api.telegram.org/file/bot{}/{}", bot_token, file_path);
//     let response = state.client
//         .get(&file_url)
//         .send()
//         .await
//         .map_err(AppError::from)?;

//     let content_type = {
//         let from_headers = response
//             .headers()
//             .get("content-type")
//             .and_then(|v| v.to_str().ok())
//             .map(String::from);

//         match from_headers {
//             Some(ct) if ct != "application/octet-stream" => ct,
//             _ => {
//                 if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
//                     "image/jpeg".to_string()
//                 } else if file_path.ends_with(".png") {
//                     "image/png".to_string()
//                 } else if file_path.ends_with(".svg") {
//                     "image/svg+xml".to_string()
//                 } else {
//                     "image/jpeg".to_string() // Default
//                 }
//             }
//         }
//     };
//     let image_bytes = response.bytes().await.map_err(AppError::from)?;

//     let cache_duration_days = 7;

//     Ok((
//         StatusCode::OK,
//         [
//             (header::CONTENT_TYPE, content_type),
//             (
//                 header::CACHE_CONTROL,
//                 format!("public, max-age={}, stale-while-revalidate=604800", 86400 * cache_duration_days)
//             ),
//             (header::ETAG, format!("\"{}\"", file_id)), // file_id is stable per photo
//         ],
//         // [(header::CONTENT_TYPE, content_type)],
//         image_bytes.to_vec(),
//     )
//         .into_response())
// }