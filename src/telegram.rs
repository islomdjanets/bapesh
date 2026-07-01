// use urlencoding::decode;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::{Sha256};

use std::{error::Error};
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