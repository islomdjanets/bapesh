use axum::{Json, extract::{FromRequestParts}, http::request::Parts, response::IntoResponse};
use hyper::{StatusCode};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use chrono::{Utc, Duration};
use serde_json::json;

use crate::{env, json::JSON, telegram};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,      // The user_id
    pub exp: usize,    // Expiration time (Unix timestamp)
    pub iat: usize,    // Issued at (Unix timestamp)
}

#[derive(Debug, Serialize)]
pub struct LoginResult {
    pub token: String,
    pub user: Option<telegram::User>,
    pub data: Option<JSON>,
    pub is_created: bool
}

impl IntoResponse for LoginResult {
    fn into_response(self) -> axum::response::Response {
        let body = Json(json!({
            "token": self.token,
            "user": self.user,
            "data": self.data,
            "is_created": self.is_created
        }));
        (StatusCode::OK, body).into_response()
    }
}

pub fn login(
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
            data: None,
            is_created: false
        };
    }

    let user = telegram::extract_user(init_data);

    if user.is_none() {
        // println!("No user extracted from init data: {:?}", init_data);

        return LoginResult {
            user: None,
            token: String::new(),
            data: None,
            is_created: false
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

    let token = issue_token(user.id);

    // 3. Return the token to the JS engine
    LoginResult {
        token: token,
        user: Some(user),
        data: None,
        is_created: true
    }
}

/// How long a token is good for. One number for every service, because the
/// client decides whether to re-login by reading `exp` out of whatever token
/// it holds, whichever service minted it.
pub const TOKEN_DAYS: i64 = 60;

/// Mints the bearer token every bapesh service accepts.
///
/// HS256 over `Claims`, keyed by the `JWT_SECRET` shared across the ecosystem —
/// which is what makes a token from prestige valid on gg_arcade, roomtour and
/// the taskers without any of them calling back. `sub` is the account id and
/// nothing else is in it: a profile in a 60-day token is stale for 59 of them.
pub fn issue_token(user_id: i64) -> String {
    let now = Utc::now();
    let expiration = now
        .checked_add_signed(Duration::days(TOKEN_DAYS))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user_id,
        iat: now.timestamp() as usize,
        exp: expiration as usize,
    };

    let secret = env::get("JWT_SECRET")
        .expect("JWT_SECRET IS NOT SETUP").into_bytes();

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&secret),
    ).expect("HS256 signing cannot fail on a valid secret")
}

/// The claims of a token this ecosystem issued, or `None` for anything else:
/// wrong secret, expired, malformed. The same check `AuthenticatedUser` makes,
/// for the handlers where a token is optional rather than required.
pub fn verify_token(token: &str) -> Option<Claims> {
    let secret = env::get("JWT_SECRET")
        .expect("JWT_SECRET IS NOT SETUP").into_bytes();

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&secret),
        &Validation::default(),
    ).ok().map(|data| data.claims)
}

/// The bearer token on a request, if one was sent. Validity is `verify_token`'s
/// business; this only finds it.
pub fn bearer(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str().ok()?
        .strip_prefix("Bearer ")
}

// ── the Prestige account ──────────────────────────────────────────────────
//
// One account per person across every project, held by the prestige service
// and reachable by three sign-ins (Telegram, Google, Apple). Projects keep
// their own `users` row — keyed by the same id — for whatever is theirs, and
// seed it from this on first sight of a token whose `sub` they do not know.

/// One way of signing in that is attached to an account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// `telegram`, `google` or `apple`.
    pub provider: String,
    /// The provider's own stable id for the person: the Telegram user id as a
    /// string, Google's `sub`, Apple's `sub`.
    pub subject: String,
    #[serde(default)]
    pub email: Option<String>,
    /// Whatever the provider told us about the person at last sign-in
    /// (`username`, `first_name`, `photo_url` for Telegram; `name`, `picture`
    /// for Google). Display data only — nothing here is a credential.
    #[serde(default)]
    pub profile: serde_json::Value,
}

/// A Prestige account, as `GET /auth/me` and `GET /internal/account/{id}`
/// serve it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// The id every project's `users.id` and every balance is keyed by. For an
    /// account that started from Telegram this *is* the Telegram user id; for
    /// one that started from Google or Apple it is allocated by prestige from a
    /// range no Telegram id can reach.
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub identities: Vec<Identity>,
}

impl Account {
    pub fn identity(&self, provider: &str) -> Option<&Identity> {
        self.identities.iter().find(|i| i.provider == provider)
    }

    /// The Telegram user id, when Telegram is one of the sign-ins. What a
    /// project needs before it may call the Bot API about this person — a
    /// profile photo, a message — since the account id is not one for
    /// accounts that started elsewhere.
    pub fn telegram_id(&self) -> Option<i64> {
        self.identity("telegram")?.subject.parse().ok()
    }

    /// The account as the `telegram::User` shape every client already reads
    /// its `/login` response into. `username` comes from the Telegram identity
    /// when there is one and is otherwise empty — a project that needs a
    /// handle derives its own, as gg_arcade does.
    pub fn as_telegram_user(&self) -> telegram::User {
        let tg = self.identity("telegram");
        let field = |k: &str| tg
            .and_then(|i| i.profile.get(k))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        telegram::User {
            id: self.id,
            first_name: self.name.clone(),
            last_name: field("last_name"),
            username: field("username"),
            language_code: self.language.clone().unwrap_or_default(),
            is_premium: false,
            allows_write_to_pm: false,
            photo_url: self.avatar_url.clone().unwrap_or_default(),
        }
    }
}

/// The Telegram login, with prestige told about it.
///
/// Validates initData against this bot's token exactly as `login` does, then
/// asks prestige for the account behind the Telegram user
/// (`POST /internal/auth/telegram`, internal secret) and hands back the token
/// *prestige* minted. That token names the account, which is the Telegram id
/// for almost everyone and is not for a Telegram that was linked to a
/// Google-first account — the case a locally minted token would get wrong.
/// It is also what gives the person a `telegram` identity to link Google or
/// Apple onto later.
///
/// Prestige unreachable falls back to the local token rather than failing
/// the login: the two are the same for a Telegram-first account, and a
/// Mini App must open when the account service is having a bad minute. The
/// fallback is logged, because it is the one path that can seat a linked
/// person on the wrong id.
pub async fn login_prestige(
    init_data: &str,
    bot_token: &str,
    client: &reqwest::Client,
    internal_secret: &str,
) -> LoginResult {
    let local = login(init_data, bot_token);
    let Some(user) = local.user.as_ref() else {
        return local;
    };

    match attest_telegram(client, internal_secret, user).await {
        Ok((token, account)) => {
            let mut profile = account.as_telegram_user();
            // What initData said about the person right now beats what the
            // account remembers from last time.
            profile.username = user.username.clone();
            profile.is_premium = user.is_premium;
            profile.allows_write_to_pm = user.allows_write_to_pm;
            if profile.language_code.is_empty() {
                profile.language_code = user.language_code.clone();
            }

            LoginResult { token, user: Some(profile), data: None, is_created: false }
        }
        Err(e) => {
            eprintln!("login: prestige did not answer for telegram user {} ({e}); local token", user.id);
            local
        }
    }
}

/// Tells prestige a Telegram user signed in with this bot, and gets the
/// account and its token back. See `login_prestige`.
pub async fn attest_telegram(
    client: &reqwest::Client,
    internal_secret: &str,
    user: &telegram::User,
) -> Result<(String, Account), String> {
    #[derive(Deserialize)]
    struct Session {
        token: String,
        user: Account,
    }

    let resp = client
        .post(format!("{}/internal/auth/telegram", crate::prestige::host()))
        .header("X-Internal-Secret", internal_secret)
        .json(user)
        .send().await
        .map_err(|e| format!("prestige unreachable: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("prestige answered {status}"));
    }

    let session: Session = resp.json().await.map_err(|e| format!("prestige answered nonsense: {e}"))?;
    Ok((session.token, session.user))
}

/// Reads an account from prestige, by id, with the internal secret.
///
/// `Ok(None)` is an id prestige has never issued — which, for a token that
/// verified against the shared secret, means the token was minted by one of
/// the legacy Telegram logins for a person prestige has not yet seen. Callers
/// treat that the way they always have: the id is a Telegram id.
pub async fn fetch_account(
    client: &reqwest::Client,
    internal_secret: &str,
    id: i64,
) -> Result<Option<Account>, String> {
    let resp = client
        .get(format!("{}/internal/account/{}", crate::prestige::host(), id))
        .header("X-Internal-Secret", internal_secret)
        .send().await
        .map_err(|e| format!("prestige unreachable: {e}"))?;

    match resp.status() {
        reqwest::StatusCode::OK => resp.json().await.map(Some).map_err(|e| format!("prestige answered nonsense: {e}")),
        reqwest::StatusCode::NOT_FOUND => Ok(None),
        status => Err(format!("prestige answered {status}")),
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

        let JWT_SECRET = env::get("JWT_SECRET")
            .expect("JWT_SECRET IS NOT SETUP").into_bytes();

        // 2. Decode and Validate the JWT
        let token_data = decode::<Claims>(
            auth_header,
            &DecodingKey::from_secret(&JWT_SECRET),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired token"))?;

        Ok(AuthenticatedUser {
            id: token_data.claims.sub,
        })
    }
}

/// `Option<AuthenticatedUser>` -- for a route a guest may call.
///
/// No Authorization header at all is `None`: a guest. A header that is there
/// but does not verify is still a 401, not a guest -- a client that presents
/// a token means to be someone, and being quietly demoted to nobody would
/// show it a feed with every like unset and no way to tell why.
impl<S> axum::extract::OptionalFromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        if parts.headers.get(axum::http::header::AUTHORIZATION).is_none() {
            return Ok(None);
        }
        <Self as FromRequestParts<S>>::from_request_parts(parts, state).await.map(Some)
    }
}