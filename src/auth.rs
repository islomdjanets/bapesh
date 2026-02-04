use std::collections::HashMap;

use axum::{Json, extract::{FromRequestParts, Query}, http::request::Parts, response::IntoResponse};
use hyper::{StatusCode};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, Duration};
use serde_json::json;

use crate::{json::JSON, telegram};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,      // The user_id
    pub exp: usize,    // Expiration time (Unix timestamp)
    pub iat: usize,    // Issued at (Unix timestamp)
}

const JWT_SECRET: &[u8] = b"your_ultra_secret_game_key_123"; // Use an Env Var in production!

#[derive(Debug, Serialize)]
pub struct LoginResult {
    token: String,
    user: Option<telegram::User>,
    // data: Option<JSON>,
    // is_created: bool
}

// #[derive(Debug, Deserialize)]
// struct UserIdPayload{
//     user_id: u64,
// }

pub async fn login(
    // Query(params): Query<HashMap<String, String>>,
    init_data: &str,
    bot_token: &str,
) -> LoginResult {

    let valid = telegram::validate_init_data(init_data, &bot_token).unwrap_or(false);
    if !valid {
        // println!("Invalid init_data");
        return LoginResult {
            user: None,
            token: String::new(),
        };
    }

    let user = telegram::extract_user(init_data);

    if user.is_none() {
        // println!("No user extracted from init data: {:?}", init_data);

        return LoginResult {
            user: None,
            token: String::new(),
        };
    }
    let user = user.unwrap();

    // let pool = state.pool.clone();
    // let data = extract_data(&user, pool).await;
    // if data.is_none() {
    //     println!("Failed to extract or create user data for user {}", user.id);
    //     return LoginResult {
    //         user: None,
    //         data: None,
    //         is_created: false
    //     };
    // }

    // let data = data.unwrap();

    // 3. Create the Cookie
    // We use the user.id as the value. 
    // let user_id_str = user.id.to_string();

    // let login_result = LoginResult {
    //     user: Some(user),
    //     token: user_id_str,
    // };

    // login_result

    let user_id = user.id;

    // 1. Create the claims
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(60))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id,
        iat: Utc::now().timestamp() as usize,
        exp: expiration as usize,
    };

    // 2. Sign the token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    ).unwrap();

    // 3. Return the token to the JS engine
    LoginResult {
        token: token,
        user: Some(user),
    }
}

pub struct AuthenticatedUser {
    pub id: i64, // Using i64 assuming your Telegram/DB IDs are integers
}

// impl<S> FromRequestParts<S> for AuthenticatedUser
// where
//     S: Send + Sync,
// {
//     type Rejection = (StatusCode, &'static str);

//     async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
//         // 1. Get the Authorization header
//         let auth_header = parts.headers
//             .get(header::AUTHORIZATION)
//             .and_then(|val| val.to_str().ok())
//             .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header"))?;

//         // 2. Check if it starts with "Bearer "
//         if !auth_header.starts_with("Bearer ") {
//             return Err((StatusCode::UNAUTHORIZED, "Invalid Authorization type"));
//         }

//         // 3. Extract the ID (the token)
//         let token = &auth_header[7..];
//         let id = token
//             .parse::<i64>()
//             .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid User ID in token"))?;

//         Ok(AuthenticatedUser { id })
//     }
// }

impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Get the Bearer token
        let auth_header = parts.headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "Missing or invalid token"))?;

        // 2. Decode and Validate the JWT
        let token_data = decode::<Claims>(
            auth_header,
            &DecodingKey::from_secret(JWT_SECRET),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        Ok(AuthenticatedUser {
            id: token_data.claims.sub,
        })
    }
}