use base64::engine::general_purpose::URL_SAFE;
use base64::Engine;

use teloxide::prelude::*;
use teloxide::types::InlineKeyboardButtonKind::WebApp;
use teloxide::types::{ InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, WebAppInfo };

use crate::env;
use crate::server::Service;

const URL: &str = "https://bc53c38f-e85d-403c-b6de-2e8bf191765d.selcdn.net";

async fn start(message: Message, bot: Bot) -> ResponseResult<()> {
    let mut params = Vec::new();
    if let Some(text) = message.text() {
        let args: Vec<&str> = text.splitn(2, ' ').collect();
        let data_str = if args.len() > 1 { args[1] } else { "" };

        let decoded_data = URL_SAFE
            .decode(data_str)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok());

        if let Some(decoded_data) = decoded_data {
            let ref_index = decoded_data.find("r=");
            let query_index = decoded_data.find("q=");
            if let Some(ref_index) = ref_index {
                let referral_id =
                    &decoded_data[ref_index + 2..query_index.unwrap_or(decoded_data.len())];
                params.push(format!("ref={}", referral_id));
            }
            if let Some(query_index) = query_index {
                let query_id = &decoded_data[query_index + 2..];
                params.push(format!("q={}", query_id));
            }
        }
    }

    let premium_user_status = message.from().map_or(false, |user| user.is_premium);
    if premium_user_status {
        params.push(format!("pr={}", premium_user_status));
    }

    let url = if params.is_empty() {
        URL.to_string()
    } else {
        format!("{}?{}", URL, params.join("&"))
    };

    // Convert the URL string to a reqwest::Url
    let url = reqwest::Url::parse(&url).expect("Invalid URL");

    // let inline_kb = InlineKeyboardMarkup::new(
    //     vec![vec![InlineKeyboardButton::url(
    //         "Open the App",
    //         url,
    //     )]]
    // );

    let mini_app_button = InlineKeyboardButton::new(
        "Open the App",
        WebApp( WebAppInfo {
            url,
        })
    );

    let keyboard = InlineKeyboardMarkup::new(vec![vec![mini_app_button]]);

    bot
        .send_message(
            message.chat.id,
            // format!("Hello! This is a test bot. You can visit the web page by clicking the button below.\n\n{}\n<a href='{}'>URL</a>", url, url)
            "Welcome to Rogarlic Beta! Collect resources, improve production and collect all of the NFT's to take advantage in actual game and MAX PROFIT"
        )
        .parse_mode(ParseMode::Html)
        // .reply_markup(inline_kb).await?;
        .reply_markup(keyboard).await?;

    Ok(())
}

pub struct Telegram {

}

impl Service for Telegram {
    fn uri(&self, request: &crate::handshake::Request) -> bool {
        todo!()
    }

    fn handler(&self, stream: &mut tokio::net::TcpStream, resources: std::sync::Arc<std::sync::Mutex<crate::server::Resources>>) {
        todo!()
    }

}

pub async fn serve() {
    let token = env::get("TOKEN").expect("TOKEN not set");

    let bot = Bot::new(token);

    teloxide::repl(bot.clone(), move |message| {
        let bot = bot.clone();
        async move {
            start(message, bot).await.log_on_error().await;
            respond(())
        }
    }).await;
}

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
