// use urlencoding::decode;
use hmac::{Hmac, Mac};
use sha2::{Sha256};

use std::{error::Error};
use serde::{Serialize, Deserialize};

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
