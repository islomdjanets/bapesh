//version 9
use std::error::Error as StdError;
use std::fmt;
use std::result;
use std::sync::Arc;
use std::str::FromStr;

use base64::{engine::general_purpose::{STANDARD,URL_SAFE_NO_PAD}, Engine};
use serde::{Deserialize, Serialize, Deserializer};

use std::borrow::Cow;
use std::collections::HashSet;
use std::marker::PhantomData;

use serde::de::{self, Visitor};
use serde::de::DeserializeOwned;
// use serde::ser::Serialize;

use crate::crypto;
use crate::jwk::{AlgorithmParameters, Jwk};
#[cfg(feature = "use_pem")]
use crate::pem::PemEncodedKey;

// Errors

/// A crate private constructor for `Error`.
pub(crate) fn new_error(kind: ErrorKind) -> Error {
    Error(Box::new(kind))
}

/// A type alias for `Result<T, jsonwebtoken::errors::Error>`.
pub type Result<T> = result::Result<T, Error>;

/// An error that can occur when encoding/decoding JWTs
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error(Box<ErrorKind>);

impl Error {
    /// Return the specific type of this error.
    pub fn kind(&self) -> &ErrorKind {
        &self.0
    }

    /// Unwrap this error into its underlying type.
    pub fn into_kind(self) -> ErrorKind {
        *self.0
    }
}

/// The specific type of an error.
///
/// This enum may grow additional variants, the `#[non_exhaustive]`
/// attribute makes sure clients don't count on exhaustive matching.
/// (Otherwise, adding a new variant could break existing code.)
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum ErrorKind {
    /// When a token doesn't have a valid JWT shape
    InvalidToken,
    /// When the signature doesn't match
    InvalidSignature,
    /// When the secret given is not a valid ECDSA key
    InvalidEcdsaKey,
    /// When the secret given is not a valid RSA key
    InvalidRsaKey(String),
    /// We could not sign with the given key
    RsaFailedSigning,
    /// When the algorithm from string doesn't match the one passed to `from_str`
    InvalidAlgorithmName,
    /// When a key is provided with an invalid format
    InvalidKeyFormat,

    // Validation errors
    /// When a claim required by the validation is not present
    MissingRequiredClaim(String),
    /// When a token’s `exp` claim indicates that it has expired
    ExpiredSignature,
    /// When a token’s `iss` claim does not match the expected issuer
    InvalidIssuer,
    /// When a token’s `aud` claim does not match one of the expected audience values
    InvalidAudience,
    /// When a token’s `sub` claim does not match one of the expected subject values
    InvalidSubject,
    /// When a token’s `nbf` claim represents a time in the future
    ImmatureSignature,
    /// When the algorithm in the header doesn't match the one passed to `decode` or the encoding/decoding key
    /// used doesn't match the alg requested
    InvalidAlgorithm,
    /// When the Validation struct does not contain at least 1 algorithm
    MissingAlgorithm,

    // 3rd party errors
    /// An error happened when decoding some base64 text
    Base64(base64::DecodeError),
    /// An error happened while serializing/deserializing JSON
    Json(Arc<serde_json::Error>),
    /// Some of the text was invalid UTF-8
    Utf8(::std::string::FromUtf8Error),
    /// Something unspecified went wrong with crypto
    Crypto(::ring::error::Unspecified),
}

impl StdError for Error {
    fn cause(&self) -> Option<&dyn StdError> {
        match &*self.0 {
            // ErrorKind::InvalidToken => None,
            // ErrorKind::InvalidSignature => None,
            // ErrorKind::InvalidEcdsaKey => None,
            // ErrorKind::RsaFailedSigning => None,
            // ErrorKind::InvalidRsaKey(_) => None,
            // ErrorKind::ExpiredSignature => None,
            // ErrorKind::MissingAlgorithm => None,
            // ErrorKind::MissingRequiredClaim(_) => None,
            // ErrorKind::InvalidIssuer => None,
            // ErrorKind::InvalidAudience => None,
            // ErrorKind::InvalidSubject => None,
            // ErrorKind::ImmatureSignature => None,
            // ErrorKind::InvalidAlgorithm => None,
            // ErrorKind::InvalidAlgorithmName => None,
            // ErrorKind::InvalidKeyFormat => None,
            ErrorKind::Base64(err) => Some(err),
            ErrorKind::Json(err) => Some(err.as_ref()),
            ErrorKind::Utf8(err) => Some(err),
            // ErrorKind::Crypto(err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self.0 {
            // ErrorKind::InvalidToken
            // | ErrorKind::InvalidSignature
            // | ErrorKind::InvalidEcdsaKey
            // | ErrorKind::ExpiredSignature
            // | ErrorKind::RsaFailedSigning
            // | ErrorKind::MissingAlgorithm
            // | ErrorKind::InvalidIssuer
            // | ErrorKind::InvalidAudience
            // | ErrorKind::InvalidSubject
            // | ErrorKind::ImmatureSignature
            // | ErrorKind::InvalidAlgorithm
            // | ErrorKind::InvalidKeyFormat
            // | ErrorKind::InvalidAlgorithmName => write!(f, "{:?}", self.0),

            ErrorKind::MissingRequiredClaim(c) => write!(f, "Missing required claim: {}", c),
            ErrorKind::InvalidRsaKey(msg) => write!(f, "RSA key invalid: {}", msg),
            ErrorKind::Json(err) => write!(f, "JSON error: {}", err),
            ErrorKind::Utf8(err) => write!(f, "UTF-8 error: {}", err),
            ErrorKind::Crypto(err) => write!(f, "Crypto error: {}", err),
            ErrorKind::Base64(err) => write!(f, "Base64 error: {}", err),
            _ => write!(f, "{:?}", self.0),
        }
    }
}

impl PartialEq for ErrorKind {
    fn eq(&self, other: &Self) -> bool {
        format!("{:?}", self) == format!("{:?}", other)
    }
}

// Equality of ErrorKind is an equivalence relation: it is reflexive, symmetric and transitive.
impl Eq for ErrorKind {}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Error {
        new_error(ErrorKind::Base64(err))
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        new_error(ErrorKind::Json(Arc::new(err)))
    }
}

impl From<::std::string::FromUtf8Error> for Error {
    fn from(err: ::std::string::FromUtf8Error) -> Error {
        new_error(ErrorKind::Utf8(err))
    }
}

impl From<::ring::error::Unspecified> for Error {
    fn from(err: ::ring::error::Unspecified) -> Error {
        new_error(ErrorKind::Crypto(err))
    }
}

impl From<::ring::error::KeyRejected> for Error {
    fn from(_err: ::ring::error::KeyRejected) -> Error {
        new_error(ErrorKind::InvalidEcdsaKey)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        new_error(kind)
    }
}

// Algorithms
#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub(crate) enum AlgorithmFamily {
    Hmac,
    Rsa,
    Ec,
    Ed,
}

/// The algorithms supported for signing/verifying JWTs
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Default, PartialEq, Eq, Hash, Copy, Clone, Serialize, Deserialize)]
pub enum Algorithm {
    /// HMAC using SHA-256
    #[default]
    HS256,
    /// HMAC using SHA-384
    HS384,
    /// HMAC using SHA-512
    HS512,

    /// ECDSA using SHA-256
    ES256,
    /// ECDSA using SHA-384
    ES384,

    /// RSASSA-PKCS1-v1_5 using SHA-256
    RS256,
    /// RSASSA-PKCS1-v1_5 using SHA-384
    RS384,
    /// RSASSA-PKCS1-v1_5 using SHA-512
    RS512,

    /// RSASSA-PSS using SHA-256
    PS256,
    /// RSASSA-PSS using SHA-384
    PS384,
    /// RSASSA-PSS using SHA-512
    PS512,

    /// Edwards-curve Digital Signature Algorithm (EdDSA)
    EdDSA,
}

impl FromStr for Algorithm {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "HS256" => Ok(Algorithm::HS256),
            "HS384" => Ok(Algorithm::HS384),
            "HS512" => Ok(Algorithm::HS512),
            "ES256" => Ok(Algorithm::ES256),
            "ES384" => Ok(Algorithm::ES384),
            "RS256" => Ok(Algorithm::RS256),
            "RS384" => Ok(Algorithm::RS384),
            "PS256" => Ok(Algorithm::PS256),
            "PS384" => Ok(Algorithm::PS384),
            "PS512" => Ok(Algorithm::PS512),
            "RS512" => Ok(Algorithm::RS512),
            "EdDSA" => Ok(Algorithm::EdDSA),
            _ => Err(ErrorKind::InvalidAlgorithmName.into()),
        }
    }
}

impl Algorithm {
    pub(crate) fn family(self) -> AlgorithmFamily {
        match self {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => AlgorithmFamily::Hmac,
            Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 | Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => AlgorithmFamily::Rsa,
            Algorithm::ES256 | Algorithm::ES384 => AlgorithmFamily::Ec,
            Algorithm::EdDSA => AlgorithmFamily::Ed,
        }
    }
}

// Headers

/// A basic JWT header, the alg defaults to HS256 and typ is automatically
/// set to `JWT`. All the other fields are optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct Header {
    /// The type of JWS: it can only be "JWT" here
    ///
    /// Defined in [RFC7515#4.1.9](https://tools.ietf.org/html/rfc7515#section-4.1.9).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>,
    /// The algorithm used
    ///
    /// Defined in [RFC7515#4.1.1](https://tools.ietf.org/html/rfc7515#section-4.1.1).
    pub alg: Algorithm,
    /// Content type
    ///
    /// Defined in [RFC7519#5.2](https://tools.ietf.org/html/rfc7519#section-5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cty: Option<String>,
    /// JSON Key URL
    ///
    /// Defined in [RFC7515#4.1.2](https://tools.ietf.org/html/rfc7515#section-4.1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jku: Option<String>,
    /// JSON Web Key
    ///
    /// Defined in [RFC7515#4.1.3](https://tools.ietf.org/html/rfc7515#section-4.1.3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwk: Option<Jwk>,
    /// Key ID
    ///
    /// Defined in [RFC7515#4.1.4](https://tools.ietf.org/html/rfc7515#section-4.1.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>,
    /// X.509 URL
    ///
    /// Defined in [RFC7515#4.1.5](https://tools.ietf.org/html/rfc7515#section-4.1.5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5u: Option<String>,
    /// X.509 certificate chain. A Vec of base64 encoded ASN.1 DER certificates.
    ///
    /// Defined in [RFC7515#4.1.6](https://tools.ietf.org/html/rfc7515#section-4.1.6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5c: Option<Vec<String>>,
    /// X.509 SHA1 certificate thumbprint
    ///
    /// Defined in [RFC7515#4.1.7](https://tools.ietf.org/html/rfc7515#section-4.1.7).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x5t: Option<String>,
    /// X.509 SHA256 certificate thumbprint
    ///
    /// Defined in [RFC7515#4.1.8](https://tools.ietf.org/html/rfc7515#section-4.1.8).
    ///
    /// This will be serialized/deserialized as "x5t#S256", as defined by the RFC.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "x5t#S256")]
    pub x5t_s256: Option<String>,
}

impl Header {
    /// Returns a JWT header with the algorithm given
    pub fn new(algorithm: Algorithm) -> Self {
        Header {
            typ: Some("JWT".to_string()),
            alg: algorithm,
            cty: None,
            jku: None,
            jwk: None,
            kid: None,
            x5u: None,
            x5c: None,
            x5t: None,
            x5t_s256: None,
        }
    }

    /// Converts an encoded part into the Header struct if possible
    pub(crate) fn from_encoded<T: AsRef<[u8]>>(encoded_part: T) -> Result<Self> {
        let decoded = b64_decode(encoded_part)?;
        Ok(serde_json::from_slice(&decoded)?)
    }

    /// Decodes the X.509 certificate chain into ASN.1 DER format.
    pub fn x5c_der(&self) -> Result<Option<Vec<Vec<u8>>>> {
        Ok(self
            .x5c
            .as_ref()
            .map(|b64_certs| {
                b64_certs.iter().map(|x| STANDARD.decode(x)).collect::<result::Result<_, _>>()
            })
            .transpose()?)
    }
}

impl Default for Header {
    /// Returns a JWT header using the default Algorithm, HS256
    fn default() -> Self {
        Header::new(Algorithm::default())
    }
}

// Serialization

pub(crate) fn b64_encode<T: AsRef<[u8]>>(input: T) -> String {
    URL_SAFE_NO_PAD.encode(input)
}

pub(crate) fn b64_decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD.decode(input).map_err(|e| e.into())
}

/// Serializes a struct to JSON and encodes it in base64
pub(crate) fn b64_encode_part<T: Serialize>(input: &T) -> Result<String> {
    let json = serde_json::to_vec(input)?;
    Ok(b64_encode(json))
}

/// This is used to decode from base64 then deserialize from JSON to several structs:
/// - The user-provided struct
/// - The ClaimsForValidation struct from this crate to run validation on
pub(crate) struct DecodedJwtPartClaims {
    b64_decoded: Vec<u8>,
}

impl DecodedJwtPartClaims {
    pub fn from_jwt_part_claims(encoded_jwt_part_claims: impl AsRef<[u8]>) -> Result<Self> {
        Ok(Self { b64_decoded: b64_decode(encoded_jwt_part_claims)? })
    }

    pub fn deserialize<'a, T: Deserialize<'a>>(&'a self) -> Result<T> {
        Ok(serde_json::from_slice(&self.b64_decoded)?)
    }
}

// Validation

/// Contains the various validations that are applied after decoding a JWT.
///
/// All time validation happen on UTC timestamps as seconds.
///
/// ```rust
/// use jsonwebtoken::{Validation, Algorithm};
///
/// let mut validation = Validation::new(Algorithm::HS256);
/// validation.leeway = 5;
/// // Setting audience
/// validation.set_audience(&["Me"]); // a single string
/// validation.set_audience(&["Me", "You"]); // array of strings
/// // or issuer
/// validation.set_issuer(&["Me"]); // a single string
/// validation.set_issuer(&["Me", "You"]); // array of strings
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validation {
    /// Which claims are required to be present before starting the validation.
    /// This does not interact with the various `validate_*`. If you remove `exp` from that list, you still need
    /// to set `validate_exp` to `false`.
    /// The only value that will be used are "exp", "nbf", "aud", "iss", "sub". Anything else will be ignored.
    ///
    /// Defaults to `{"exp"}`
    pub required_spec_claims: HashSet<String>,
    /// Add some leeway (in seconds) to the `exp` and `nbf` validation to
    /// account for clock skew.
    ///
    /// Defaults to `60`.
    pub leeway: u64,
    /// Whether to validate the `exp` field.
    ///
    /// It will return an error if the time in the `exp` field is past.
    ///
    /// Defaults to `true`.
    pub validate_exp: bool,
    /// Whether to validate the `nbf` field.
    ///
    /// It will return an error if the current timestamp is before the time in the `nbf` field.
    ///
    /// Defaults to `false`.
    pub validate_nbf: bool,
    /// Whether to validate the `aud` field.
    ///
    /// It will return an error if the `aud` field is not a member of the audience provided.
    ///
    /// Defaults to `true`. Very insecure to turn this off. Only do this if you know what you are doing.
    pub validate_aud: bool,
    /// Validation will check that the `aud` field is a member of the
    /// audience provided and will error otherwise.
    /// Use `set_audience` to set it
    ///
    /// Defaults to `None`.
    pub aud: Option<HashSet<String>>,
    /// If it contains a value, the validation will check that the `iss` field is a member of the
    /// iss provided and will error otherwise.
    /// Use `set_issuer` to set it
    ///
    /// Defaults to `None`.
    pub iss: Option<HashSet<String>>,
    /// If it contains a value, the validation will check that the `sub` field is the same as the
    /// one provided and will error otherwise.
    ///
    /// Defaults to `None`.
    pub sub: Option<String>,
    /// The validation will check that the `alg` of the header is contained
    /// in the ones provided and will error otherwise. Will error if it is empty.
    ///
    /// Defaults to `vec![Algorithm::HS256]`.
    pub algorithms: Vec<Algorithm>,

    /// Whether to validate the JWT signature. Very insecure to turn that off
    pub(crate) validate_signature: bool,
}

impl Validation {
    /// Create a default validation setup allowing the given alg
    pub fn new(alg: Algorithm) -> Validation {
        let mut required_claims = HashSet::with_capacity(1);
        required_claims.insert("exp".to_owned());

        Validation {
            required_spec_claims: required_claims,
            algorithms: vec![alg],
            leeway: 60,

            validate_exp: true,
            validate_nbf: false,
            validate_aud: true,

            iss: None,
            sub: None,
            aud: None,

            validate_signature: true,
        }
    }

    /// `aud` is a collection of one or more acceptable audience members
    /// The simple usage is `set_audience(&["some aud name"])`
    pub fn set_audience<T: ToString>(&mut self, items: &[T]) {
        self.aud = Some(items.iter().map(|x| x.to_string()).collect())
    }

    /// `iss` is a collection of one or more acceptable issuers members
    /// The simple usage is `set_issuer(&["some iss name"])`
    pub fn set_issuer<T: ToString>(&mut self, items: &[T]) {
        self.iss = Some(items.iter().map(|x| x.to_string()).collect())
    }

    /// Which claims are required to be present for this JWT to be considered valid.
    /// The only values that will be considered are "exp", "nbf", "aud", "iss", "sub".
    /// The simple usage is `set_required_spec_claims(&["exp", "nbf"])`.
    /// If you want to have an empty set, do not use this function - set an empty set on the struct
    /// param directly.
    pub fn set_required_spec_claims<T: ToString>(&mut self, items: &[T]) {
        self.required_spec_claims = items.iter().map(|x| x.to_string()).collect();
    }

    /// Whether to validate the JWT cryptographic signature.
    /// Disabling validation is dangerous, only do it if you know what you're doing.
    /// With validation disabled you should not trust any of the values of the claims.
    pub fn insecure_disable_signature_validation(&mut self) {
        self.validate_signature = false;
    }
}

impl Default for Validation {
    fn default() -> Self {
        Self::new(Algorithm::HS256)
    }
}

/// Gets the current timestamp in the format expected by JWTs.
#[cfg(not(all(target_arch = "wasm32", not(any(target_os = "emscripten", target_os = "wasi")))))]
#[must_use]
pub fn get_current_timestamp() -> u64 {
    let start = std::time::SystemTime::now();
    start.duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_secs()
}

/// Gets the current timestamp in the format expected by JWTs.
#[cfg(all(target_arch = "wasm32", not(any(target_os = "emscripten", target_os = "wasi"))))]
#[must_use]
pub fn get_current_timestamp() -> u64 {
    js_sys::Date::new_0().get_time() as u64 / 1000
}

#[derive(Deserialize)]
pub(crate) struct ClaimsForValidation<'a> {
    #[serde(deserialize_with = "numeric_type", default)]
    exp: TryParse<u64>,
    #[serde(deserialize_with = "numeric_type", default)]
    nbf: TryParse<u64>,
    #[serde(borrow)]
    sub: TryParse<Cow<'a, str>>,
    #[serde(borrow)]
    iss: TryParse<Issuer<'a>>,
    #[serde(borrow)]
    aud: TryParse<Audience<'a>>,
}
#[derive(Debug)]
enum TryParse<T> {
    Parsed(T),
    FailedToParse,
    NotPresent,
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for TryParse<T> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer) {
            Ok(Some(value)) => TryParse::Parsed(value),
            Ok(None) => TryParse::NotPresent,
            Err(_) => TryParse::FailedToParse,
        })
    }
}
impl<T> Default for TryParse<T> {
    fn default() -> Self {
        Self::NotPresent
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience<'a> {
    Single(#[serde(borrow)] Cow<'a, str>),
    Multiple(#[serde(borrow)] HashSet<BorrowedCowIfPossible<'a>>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Issuer<'a> {
    Single(#[serde(borrow)] Cow<'a, str>),
    Multiple(#[serde(borrow)] HashSet<BorrowedCowIfPossible<'a>>),
}

/// Usually #[serde(borrow)] on `Cow` enables deserializing with no allocations where
/// possible (no escapes in the original str) but it does not work on e.g. `HashSet<Cow<str>>`
/// We use this struct in this case.
#[derive(Deserialize, PartialEq, Eq, Hash)]
struct BorrowedCowIfPossible<'a>(#[serde(borrow)] Cow<'a, str>);
impl std::borrow::Borrow<str> for BorrowedCowIfPossible<'_> {
    fn borrow(&self) -> &str {
        &self.0
    }
}

fn is_subset(reference: &HashSet<String>, given: &HashSet<BorrowedCowIfPossible<'_>>) -> bool {
    // Check that intersection is non-empty, favoring iterating on smallest
    if reference.len() < given.len() {
        reference.iter().any(|a| given.contains(&**a))
    } else {
        given.iter().any(|a| reference.contains(&*a.0))
    }
}

pub(crate) fn validate(claims: ClaimsForValidation, options: &Validation) -> Result<()> {
    for required_claim in &options.required_spec_claims {
        let present = match required_claim.as_str() {
            "exp" => matches!(claims.exp, TryParse::Parsed(_)),
            "sub" => matches!(claims.sub, TryParse::Parsed(_)),
            "iss" => matches!(claims.iss, TryParse::Parsed(_)),
            "aud" => matches!(claims.aud, TryParse::Parsed(_)),
            "nbf" => matches!(claims.nbf, TryParse::Parsed(_)),
            _ => continue,
        };

        if !present {
            return Err(new_error(ErrorKind::MissingRequiredClaim(required_claim.clone())));
        }
    }

    if options.validate_exp || options.validate_nbf {
        let now = get_current_timestamp();

        if matches!(claims.exp, TryParse::Parsed(exp) if options.validate_exp && exp < now - options.leeway) {
            return Err(new_error(ErrorKind::ExpiredSignature));
        }

        if matches!(claims.nbf, TryParse::Parsed(nbf) if options.validate_nbf && nbf > now + options.leeway) {
            return Err(new_error(ErrorKind::ImmatureSignature));
        }
    }

    if let (TryParse::Parsed(sub), Some(correct_sub)) = (claims.sub, options.sub.as_deref()) {
        if sub != correct_sub {
            return Err(new_error(ErrorKind::InvalidSubject));
        }
    }

    match (claims.iss, options.iss.as_ref()) {
        (TryParse::Parsed(Issuer::Single(iss)), Some(correct_iss)) => {
            if !correct_iss.contains(&*iss) {
                return Err(new_error(ErrorKind::InvalidIssuer));
            }
        }
        (TryParse::Parsed(Issuer::Multiple(iss)), Some(correct_iss)) => {
            if !is_subset(correct_iss, &iss) {
                return Err(new_error(ErrorKind::InvalidIssuer));
            }
        }
        _ => {}
    }

    if !options.validate_aud {
        return Ok(());
    }
    match (claims.aud, options.aud.as_ref()) {
        // Each principal intended to process the JWT MUST
        // identify itself with a value in the audience claim. If the principal
        // processing the claim does not identify itself with a value in the
        // "aud" claim when this claim is present, then the JWT MUST be
        //  rejected.
        (TryParse::Parsed(_), None) => {
            return Err(new_error(ErrorKind::InvalidAudience));
        }
        (TryParse::Parsed(Audience::Single(aud)), Some(correct_aud)) => {
            if !correct_aud.contains(&*aud) {
                return Err(new_error(ErrorKind::InvalidAudience));
            }
        }
        (TryParse::Parsed(Audience::Multiple(aud)), Some(correct_aud)) => {
            if !is_subset(correct_aud, &aud) {
                return Err(new_error(ErrorKind::InvalidAudience));
            }
        }
        _ => {}
    }

    Ok(())
}

fn numeric_type<'de, D>(deserializer: D) -> std::result::Result<TryParse<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct NumericType(PhantomData<fn() -> TryParse<u64>>);

    impl<'de> Visitor<'de> for NumericType {
        type Value = TryParse<u64>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("A NumericType that can be reasonably coerced into a u64")
        }

        fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            if value.is_finite() && value >= 0.0 && value < (u64::MAX as f64) {
                Ok(TryParse::Parsed(value.round() as u64))
            } else {
                Err(serde::de::Error::custom("NumericType must be representable as a u64"))
            }
        }

        fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(TryParse::Parsed(value))
        }
    }

    match deserializer.deserialize_any(NumericType(PhantomData)) {
        Ok(ok) => Ok(ok),
        Err(_) => Ok(TryParse::FailedToParse),
    }
}

// remains encoding decoding and crypto

// Decoding

/// The return type of a successful call to [decode](fn.decode.html).
#[derive(Debug)]
pub struct TokenData<T> {
    /// The decoded JWT header
    pub header: Header,
    /// The decoded JWT claims
    pub claims: T,
}

impl<T> Clone for TokenData<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self { header: self.header.clone(), claims: self.claims.clone() }
    }
}

/// Takes the result of a rsplit and ensure we only get 2 parts
/// Errors if we don't
macro_rules! expect_two {
    ($iter:expr) => {{
        let mut i = $iter;
        match (i.next(), i.next(), i.next()) {
            (Some(first), Some(second), None) => (first, second),
            _ => return Err(new_error(ErrorKind::InvalidToken)),
        }
    }};
}

#[derive(Clone)]
pub(crate) enum DecodingKeyKind {
    SecretOrDer(Vec<u8>),
    RsaModulusExponent { n: Vec<u8>, e: Vec<u8> },
}

/// All the different kind of keys we can use to decode a JWT.
/// This key can be re-used so make sure you only initialize it once if you can for better performance.
#[derive(Clone)]
pub struct DecodingKey {
    pub(crate) family: AlgorithmFamily,
    pub(crate) kind: DecodingKeyKind,
}

impl DecodingKey {
    /// If you're using HMAC, use this.
    pub fn from_secret(secret: &[u8]) -> Self {
        DecodingKey {
            family: AlgorithmFamily::Hmac,
            kind: DecodingKeyKind::SecretOrDer(secret.to_vec()),
        }
    }

    /// If you're using HMAC with a base64 encoded secret, use this.
    pub fn from_base64_secret(secret: &str) -> Result<Self> {
        let out = STANDARD.decode(secret)?;
        Ok(DecodingKey { family: AlgorithmFamily::Hmac, kind: DecodingKeyKind::SecretOrDer(out) })
    }

    /// If you are loading a public RSA key in a PEM format, use this.
    /// Only exists if the feature `use_pem` is enabled.
    #[cfg(feature = "use_pem")]
    pub fn from_rsa_pem(key: &[u8]) -> Result<Self> {
        let pem_key = PemEncodedKey::new(key)?;
        let content = pem_key.as_rsa_key()?;
        Ok(DecodingKey {
            family: AlgorithmFamily::Rsa,
            kind: DecodingKeyKind::SecretOrDer(content.to_vec()),
        })
    }

    /// If you have (n, e) RSA public key components as strings, use this.
    pub fn from_rsa_components(modulus: &str, exponent: &str) -> Result<Self> {
        let n = b64_decode(modulus)?;
        let e = b64_decode(exponent)?;
        Ok(DecodingKey {
            family: AlgorithmFamily::Rsa,
            kind: DecodingKeyKind::RsaModulusExponent { n, e },
        })
    }

    /// If you have (n, e) RSA public key components already decoded, use this.
    pub fn from_rsa_raw_components(modulus: &[u8], exponent: &[u8]) -> Self {
        DecodingKey {
            family: AlgorithmFamily::Rsa,
            kind: DecodingKeyKind::RsaModulusExponent { n: modulus.to_vec(), e: exponent.to_vec() },
        }
    }

    /// If you have a ECDSA public key in PEM format, use this.
    /// Only exists if the feature `use_pem` is enabled.
    #[cfg(feature = "use_pem")]
    pub fn from_ec_pem(key: &[u8]) -> Result<Self> {
        let pem_key = PemEncodedKey::new(key)?;
        let content = pem_key.as_ec_public_key()?;
        Ok(DecodingKey {
            family: AlgorithmFamily::Ec,
            kind: DecodingKeyKind::SecretOrDer(content.to_vec()),
        })
    }

    /// If you have (x,y) ECDSA key components
    pub fn from_ec_components(x: &str, y: &str) -> Result<Self> {
        let x_cmp = b64_decode(x)?;
        let y_cmp = b64_decode(y)?;

        let mut public_key = Vec::with_capacity(1 + x.len() + y.len());
        public_key.push(0x04);
        public_key.extend_from_slice(&x_cmp);
        public_key.extend_from_slice(&y_cmp);

        Ok(DecodingKey {
            family: AlgorithmFamily::Ec,
            kind: DecodingKeyKind::SecretOrDer(public_key),
        })
    }

    /// If you have a EdDSA public key in PEM format, use this.
    /// Only exists if the feature `use_pem` is enabled.
    #[cfg(feature = "use_pem")]
    pub fn from_ed_pem(key: &[u8]) -> Result<Self> {
        let pem_key = PemEncodedKey::new(key)?;
        let content = pem_key.as_ed_public_key()?;
        Ok(DecodingKey {
            family: AlgorithmFamily::Ed,
            kind: DecodingKeyKind::SecretOrDer(content.to_vec()),
        })
    }

    /// If you know what you're doing and have a RSA DER encoded public key, use this.
    pub fn from_rsa_der(der: &[u8]) -> Self {
        DecodingKey {
            family: AlgorithmFamily::Rsa,
            kind: DecodingKeyKind::SecretOrDer(der.to_vec()),
        }
    }

    /// If you know what you're doing and have a RSA EC encoded public key, use this.
    pub fn from_ec_der(der: &[u8]) -> Self {
        DecodingKey {
            family: AlgorithmFamily::Ec,
            kind: DecodingKeyKind::SecretOrDer(der.to_vec()),
        }
    }

    /// If you know what you're doing and have a Ed DER encoded public key, use this.
    pub fn from_ed_der(der: &[u8]) -> Self {
        DecodingKey {
            family: AlgorithmFamily::Ed,
            kind: DecodingKeyKind::SecretOrDer(der.to_vec()),
        }
    }

    /// From x part (base64 encoded) of the JWK encoding
    pub fn from_ed_components(x: &str) -> Result<Self> {
        let x_decoded = b64_decode(x)?;
        Ok(DecodingKey {
            family: AlgorithmFamily::Ed,
            kind: DecodingKeyKind::SecretOrDer(x_decoded),
        })
    }

    /// If you have a key in Jwk format
    pub fn from_jwk(jwk: &Jwk) -> Result<Self> {
        match &jwk.algorithm {
            AlgorithmParameters::RSA(params) => {
                DecodingKey::from_rsa_components(&params.n, &params.e)
            }
            AlgorithmParameters::EllipticCurve(params) => {
                DecodingKey::from_ec_components(&params.x, &params.y)
            }
            AlgorithmParameters::OctetKeyPair(params) => DecodingKey::from_ed_components(&params.x),
            AlgorithmParameters::OctetKey(params) => {
                let out = b64_decode(&params.value)?;
                Ok(DecodingKey {
                    family: AlgorithmFamily::Hmac,
                    kind: DecodingKeyKind::SecretOrDer(out),
                })
            }
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        match &self.kind {
            DecodingKeyKind::SecretOrDer(b) => b,
            DecodingKeyKind::RsaModulusExponent { .. } => unreachable!(),
        }
    }
}

/// Verify signature of a JWT, and return header object and raw payload
///
/// If the token or its signature is invalid, it will return an error.
fn verify_signature<'a>(
    token: &'a str,
    key: &DecodingKey,
    validation: &Validation,
) -> Result<(Header, &'a str)> {
    if validation.validate_signature && validation.algorithms.is_empty() {
        return Err(new_error(ErrorKind::MissingAlgorithm));
    }

    if validation.validate_signature {
        for alg in &validation.algorithms {
            if key.family != alg.family() {
                return Err(new_error(ErrorKind::InvalidAlgorithm));
            }
        }
    }

    let (signature, message) = expect_two!(token.rsplitn(2, '.'));
    let (payload, header) = expect_two!(message.rsplitn(2, '.'));
    let header = Header::from_encoded(header)?;

    if validation.validate_signature && !validation.algorithms.contains(&header.alg) {
        return Err(new_error(ErrorKind::InvalidAlgorithm));
    }

    if validation.validate_signature && !crypto::verify(signature, message.as_bytes(), key, header.alg)? {
        return Err(new_error(ErrorKind::InvalidSignature));
    }

    Ok((header, payload))
}

/// Decode and validate a JWT
///
/// If the token or its signature is invalid or the claims fail validation, it will return an error.
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct Claims {
///    sub: String,
///    company: String
/// }
///
/// let token = "a.jwt.token".to_string();
/// // Claims is a struct that implements Deserialize
/// let token_message = decode::<Claims>(&token, &DecodingKey::from_secret("secret".as_ref()), &Validation::new(Algorithm::HS256));
/// ```
pub fn decode<T: DeserializeOwned>(
    token: &str,
    key: &DecodingKey,
    validation: &Validation,
) -> Result<TokenData<T>> {
    match verify_signature(token, key, validation) {
        Err(e) => Err(e),
        Ok((header, claims)) => {
            let decoded_claims = DecodedJwtPartClaims::from_jwt_part_claims(claims)?;
            let claims = decoded_claims.deserialize()?;
            validate(decoded_claims.deserialize()?, validation)?;

            Ok(TokenData { header, claims })
        }
    }
}

/// Decode a JWT without any signature verification/validations and return its [Header](struct.Header.html).
///
/// If the token has an invalid format (ie 3 parts separated by a `.`), it will return an error.
///
/// ```rust
/// use jsonwebtoken::decode_header;
///
/// let token = "a.jwt.token".to_string();
/// let header = decode_header(&token);
/// ```
pub fn decode_header(token: &str) -> Result<Header> {
    let (_, message) = expect_two!(token.rsplitn(2, '.'));
    let (_, header) = expect_two!(message.rsplitn(2, '.'));
    Header::from_encoded(header)
}

// Encoding

/// A key to encode a JWT with. Can be a secret, a PEM-encoded key or a DER-encoded key.
/// This key can be re-used so make sure you only initialize it once if you can for better performance.
#[derive(Clone)]
pub struct EncodingKey {
    pub(crate) family: AlgorithmFamily,
    content: Vec<u8>,
}

impl EncodingKey {
    /// If you're using a HMAC secret that is not base64, use that.
    pub fn from_secret(secret: &[u8]) -> Self {
        EncodingKey { family: AlgorithmFamily::Hmac, content: secret.to_vec() }
    }

    /// If you have a base64 HMAC secret, use that.
    pub fn from_base64_secret(secret: &str) -> Result<Self> {
        let out = STANDARD.decode(secret)?;
        Ok(EncodingKey { family: AlgorithmFamily::Hmac, content: out })
    }

    /// If you are loading a RSA key from a .pem file.
    /// This errors if the key is not a valid RSA key.
    /// Only exists if the feature `use_pem` is enabled.
    ///
    /// # NOTE
    ///
    /// According to the [ring doc](https://docs.rs/ring/latest/ring/signature/struct.RsaKeyPair.html#method.from_pkcs8),
    /// the key should be at least 2047 bits.
    ///
    #[cfg(feature = "use_pem")]
    pub fn from_rsa_pem(key: &[u8]) -> Result<Self> {
        let pem_key = PemEncodedKey::new(key)?;
        let content = pem_key.as_rsa_key()?;
        Ok(EncodingKey { family: AlgorithmFamily::Rsa, content: content.to_vec() })
    }

    /// If you are loading a ECDSA key from a .pem file
    /// This errors if the key is not a valid private EC key
    /// Only exists if the feature `use_pem` is enabled.
    ///
    /// # NOTE
    ///
    /// The key should be in PKCS#8 form.
    ///
    /// You can generate a key with the following:
    ///
    /// ```sh
    /// openssl ecparam -genkey -noout -name prime256v1 \
    ///     | openssl pkcs8 -topk8 -nocrypt -out ec-private.pem
    /// ```
    #[cfg(feature = "use_pem")]
    pub fn from_ec_pem(key: &[u8]) -> Result<Self> {
        let pem_key = PemEncodedKey::new(key)?;
        let content = pem_key.as_ec_private_key()?;
        Ok(EncodingKey { family: AlgorithmFamily::Ec, content: content.to_vec() })
    }

    /// If you are loading a EdDSA key from a .pem file
    /// This errors if the key is not a valid private Ed key
    /// Only exists if the feature `use_pem` is enabled.
    #[cfg(feature = "use_pem")]
    pub fn from_ed_pem(key: &[u8]) -> Result<Self> {
        let pem_key = PemEncodedKey::new(key)?;
        let content = pem_key.as_ed_private_key()?;
        Ok(EncodingKey { family: AlgorithmFamily::Ed, content: content.to_vec() })
    }

    /// If you know what you're doing and have the DER-encoded key, for RSA only
    pub fn from_rsa_der(der: &[u8]) -> Self {
        EncodingKey { family: AlgorithmFamily::Rsa, content: der.to_vec() }
    }

    /// If you know what you're doing and have the DER-encoded key, for ECDSA
    pub fn from_ec_der(der: &[u8]) -> Self {
        EncodingKey { family: AlgorithmFamily::Ec, content: der.to_vec() }
    }

    /// If you know what you're doing and have the DER-encoded key, for EdDSA
    pub fn from_ed_der(der: &[u8]) -> Self {
        EncodingKey { family: AlgorithmFamily::Ed, content: der.to_vec() }
    }

    pub(crate) fn inner(&self) -> &[u8] {
        &self.content
    }
}

/// Encode the header and claims given and sign the payload using the algorithm from the header and the key.
/// If the algorithm given is RSA or EC, the key needs to be in the PEM format.
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use jsonwebtoken::{encode, Algorithm, Header, EncodingKey};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct Claims {
///    sub: String,
///    company: String
/// }
///
/// let my_claims = Claims {
///     sub: "b@b.com".to_owned(),
///     company: "ACME".to_owned()
/// };
///
/// // my_claims is a struct that implements Serialize
/// // This will create a JWT using HS256 as algorithm
/// let token = encode(&Header::default(), &my_claims, &EncodingKey::from_secret("secret".as_ref())).unwrap();
/// ```
pub fn encode<T: Serialize>(header: &Header, claims: &T, key: &EncodingKey) -> Result<String> {
    if key.family != header.alg.family() {
        return Err(new_error(ErrorKind::InvalidAlgorithm));
    }
    let encoded_header = b64_encode_part(header)?;
    let encoded_claims = b64_encode_part(claims)?;
    let message = [encoded_header, encoded_claims].join(".");
    let signature = crypto::sign(message.as_bytes(), key, header.alg)?;

    Ok([message, signature].join("."))
}

// pem

// /// Supported PEM files for EC and RSA Public and Private Keys
// #[derive(Debug, PartialEq)]
// enum PemType {
//     EcPublic,
//     EcPrivate,
//     RsaPublic,
//     RsaPrivate,
//     EdPublic,
//     EdPrivate,
// }
//
// #[derive(Debug, PartialEq)]
// enum Standard {
//     // Only for RSA
//     Pkcs1,
//     // RSA/EC
//     Pkcs8,
// }
//
// #[derive(Debug, PartialEq)]
// enum Classification {
//     Ec,
//     Ed,
//     Rsa,
// }
//
// /// The return type of a successful PEM encoded key with `decode_pem`
// ///
// /// This struct gives a way to parse a string to a key for use in jsonwebtoken.
// /// A struct is necessary as it provides the lifetime of the key
// ///
// /// PEM public private keys are encoded PKCS#1 or PKCS#8
// /// You will find that with PKCS#8 RSA keys that the PKCS#1 content
// /// is embedded inside. This is what is provided to ring via `Key::Der`
// /// For EC keys, they are always PKCS#8 on the outside but like RSA keys
// /// EC keys contain a section within that ultimately has the configuration
// /// that ring uses.
// /// Documentation about these formats is at
// /// PKCS#1: https://tools.ietf.org/html/rfc8017
// /// PKCS#8: https://tools.ietf.org/html/rfc5958
// #[derive(Debug)]
// pub(crate) struct PemEncodedKey {
//     content: Vec<u8>,
//     asn1: Vec<simple_asn1::ASN1Block>,
//     pem_type: PemType,
//     standard: Standard,
// }
//
// impl PemEncodedKey {
//     /// Read the PEM file for later key use
//     pub fn new(input: &[u8]) -> Result<PemEncodedKey> {
//         match pem::parse(input) {
//             Ok(content) => {
//                 let asn1_content = match simple_asn1::from_der(content.contents()) {
//                     Ok(asn1) => asn1,
//                     Err(_) => return Err(ErrorKind::InvalidKeyFormat.into()),
//                 };
//
//                 match content.tag() {
//                     // This handles a PKCS#1 RSA Private key
//                     "RSA PRIVATE KEY" => Ok(PemEncodedKey {
//                         content: content.into_contents(),
//                         asn1: asn1_content,
//                         pem_type: PemType::RsaPrivate,
//                         standard: Standard::Pkcs1,
//                     }),
//                     "RSA PUBLIC KEY" => Ok(PemEncodedKey {
//                         content: content.into_contents(),
//                         asn1: asn1_content,
//                         pem_type: PemType::RsaPublic,
//                         standard: Standard::Pkcs1,
//                     }),
//
//                     // No "EC PRIVATE KEY"
//                     // https://security.stackexchange.com/questions/84327/converting-ecc-private-key-to-pkcs1-format
//                     // "there is no such thing as a "PKCS#1 format" for elliptic curve (EC) keys"
//
//                     // This handles PKCS#8 certificates and public & private keys
//                     tag @ "PRIVATE KEY" | tag @ "PUBLIC KEY" | tag @ "CERTIFICATE" => {
//                         match classify_pem(&asn1_content) {
//                             Some(c) => {
//                                 let is_private = tag == "PRIVATE KEY";
//                                 let pem_type = match c {
//                                     Classification::Ec => {
//                                         if is_private {
//                                             PemType::EcPrivate
//                                         } else {
//                                             PemType::EcPublic
//                                         }
//                                     }
//                                     Classification::Ed => {
//                                         if is_private {
//                                             PemType::EdPrivate
//                                         } else {
//                                             PemType::EdPublic
//                                         }
//                                     }
//                                     Classification::Rsa => {
//                                         if is_private {
//                                             PemType::RsaPrivate
//                                         } else {
//                                             PemType::RsaPublic
//                                         }
//                                     }
//                                 };
//                                 Ok(PemEncodedKey {
//                                     content: content.into_contents(),
//                                     asn1: asn1_content,
//                                     pem_type,
//                                     standard: Standard::Pkcs8,
//                                 })
//                             }
//                             None => Err(ErrorKind::InvalidKeyFormat.into()),
//                         }
//                     }
//
//                     // Unknown/unsupported type
//                     _ => Err(ErrorKind::InvalidKeyFormat.into()),
//                 }
//             }
//             Err(_) => Err(ErrorKind::InvalidKeyFormat.into()),
//         }
//     }
//
//     /// Can only be PKCS8
//     pub fn as_ec_private_key(&self) -> Result<&[u8]> {
//         match self.standard {
//             Standard::Pkcs1 => Err(ErrorKind::InvalidKeyFormat.into()),
//             Standard::Pkcs8 => match self.pem_type {
//                 PemType::EcPrivate => Ok(self.content.as_slice()),
//                 _ => Err(ErrorKind::InvalidKeyFormat.into()),
//             },
//         }
//     }
//
//     /// Can only be PKCS8
//     pub fn as_ec_public_key(&self) -> Result<&[u8]> {
//         match self.standard {
//             Standard::Pkcs1 => Err(ErrorKind::InvalidKeyFormat.into()),
//             Standard::Pkcs8 => match self.pem_type {
//                 PemType::EcPublic => extract_first_bitstring(&self.asn1),
//                 _ => Err(ErrorKind::InvalidKeyFormat.into()),
//             },
//         }
//     }
//
//     /// Can only be PKCS8
//     pub fn as_ed_private_key(&self) -> Result<&[u8]> {
//         match self.standard {
//             Standard::Pkcs1 => Err(ErrorKind::InvalidKeyFormat.into()),
//             Standard::Pkcs8 => match self.pem_type {
//                 PemType::EdPrivate => Ok(self.content.as_slice()),
//                 _ => Err(ErrorKind::InvalidKeyFormat.into()),
//             },
//         }
//     }
//
//     /// Can only be PKCS8
//     pub fn as_ed_public_key(&self) -> Result<&[u8]> {
//         match self.standard {
//             Standard::Pkcs1 => Err(ErrorKind::InvalidKeyFormat.into()),
//             Standard::Pkcs8 => match self.pem_type {
//                 PemType::EdPublic => extract_first_bitstring(&self.asn1),
//                 _ => Err(ErrorKind::InvalidKeyFormat.into()),
//             },
//         }
//     }
//
//     /// Can be PKCS1 or PKCS8
//     pub fn as_rsa_key(&self) -> Result<&[u8]> {
//         match self.standard {
//             Standard::Pkcs1 => Ok(self.content.as_slice()),
//             Standard::Pkcs8 => match self.pem_type {
//                 PemType::RsaPrivate => extract_first_bitstring(&self.asn1),
//                 PemType::RsaPublic => extract_first_bitstring(&self.asn1),
//                 _ => Err(ErrorKind::InvalidKeyFormat.into()),
//             },
//         }
//     }
// }
//
// // This really just finds and returns the first bitstring or octet string
// // Which is the x coordinate for EC public keys
// // And the DER contents of an RSA key
// // Though PKCS#11 keys shouldn't have anything else.
// // It will get confusing with certificates.
// fn extract_first_bitstring(asn1: &[simple_asn1::ASN1Block]) -> Result<&[u8]> {
//     for asn1_entry in asn1.iter() {
//         match asn1_entry {
//             simple_asn1::ASN1Block::Sequence(_, entries) => {
//                 if let Ok(result) = extract_first_bitstring(entries) {
//                     return Ok(result);
//                 }
//             }
//             simple_asn1::ASN1Block::BitString(_, _, value) => {
//                 return Ok(value.as_ref());
//             }
//             simple_asn1::ASN1Block::OctetString(_, value) => {
//                 return Ok(value.as_ref());
//             }
//             _ => (),
//         }
//     }
//
//     Err(ErrorKind::InvalidEcdsaKey.into())
// }
//
// /// Find whether this is EC, RSA, or Ed
// fn classify_pem(asn1: &[simple_asn1::ASN1Block]) -> Option<Classification> {
//     // These should be constant but the macro requires
//     // #![feature(const_vec_new)]
//     let ec_public_key_oid = simple_asn1::oid!(1, 2, 840, 10_045, 2, 1);
//     let rsa_public_key_oid = simple_asn1::oid!(1, 2, 840, 113_549, 1, 1, 1);
//     let ed25519_oid = simple_asn1::oid!(1, 3, 101, 112);
//
//     for asn1_entry in asn1.iter() {
//         match asn1_entry {
//             simple_asn1::ASN1Block::Sequence(_, entries) => {
//                 if let Some(classification) = classify_pem(entries) {
//                     return Some(classification);
//                 }
//             }
//             simple_asn1::ASN1Block::ObjectIdentifier(_, oid) => {
//                 if oid == ec_public_key_oid {
//                     return Some(Classification::Ec);
//                 }
//                 if oid == rsa_public_key_oid {
//                     return Some(Classification::Rsa);
//                 }
//                 if oid == ed25519_oid {
//                     return Some(Classification::Ed);
//                 }
//             }
//             _ => {}
//         }
//     }
//     None
// }
