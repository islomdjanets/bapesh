use ring::{rand, signature, hmac};
use ring::constant_time::verify_slices_are_equal;

use crate::jwt::{Result,ErrorKind,Algorithm,b64_encode,b64_decode, EncodingKey, DecodingKeyKind, DecodingKey};


// ECDSA
//
/// Only used internally when validating EC, to map from our enum to the Ring EcdsaVerificationAlgorithm structs.
pub(crate) fn alg_to_ec_verification_ecdsa(
    alg: Algorithm,
) -> &'static signature::EcdsaVerificationAlgorithm {
    match alg {
        Algorithm::ES256 => &signature::ECDSA_P256_SHA256_FIXED,
        Algorithm::ES384 => &signature::ECDSA_P384_SHA384_FIXED,
        _ => unreachable!("Tried to get EC alg for a non-EC algorithm"),
    }
}

/// Only used internally when signing EC, to map from our enum to the Ring EcdsaVerificationAlgorithm structs.
pub(crate) fn alg_to_ec_signing(alg: Algorithm) -> &'static signature::EcdsaSigningAlgorithm {
    match alg {
        Algorithm::ES256 => &signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        Algorithm::ES384 => &signature::ECDSA_P384_SHA384_FIXED_SIGNING,
        _ => unreachable!("Tried to get EC alg for a non-EC algorithm"),
    }
}

/// The actual ECDSA signing + encoding
/// The key needs to be in PKCS8 format
pub fn sign_ecdsa(
    alg: &'static signature::EcdsaSigningAlgorithm,
    key: &[u8],
    message: &[u8],
) -> Result<String> {
    let rng = rand::SystemRandom::new();
    let signing_key = signature::EcdsaKeyPair::from_pkcs8(alg, key, &rng)?;
    let out = signing_key.sign(&rng, message)?;
    Ok(b64_encode(out))
}

// EDDSA
//
/// Only used internally when signing or validating EdDSA, to map from our enum to the Ring EdDSAParameters structs.
pub(crate) fn alg_to_ec_verification_eddsa(alg: Algorithm) -> &'static signature::EdDSAParameters {
    // To support additional key subtypes, like Ed448, we would need to match on the JWK's ("crv")
    // parameter.
    match alg {
        Algorithm::EdDSA => &signature::ED25519,
        _ => unreachable!("Tried to get EdDSA alg for a non-EdDSA algorithm"),
    }
}

/// The actual EdDSA signing + encoding
/// The key needs to be in PKCS8 format
pub fn sign_eddsa(key: &[u8], message: &[u8]) -> Result<String> {
    let signing_key = signature::Ed25519KeyPair::from_pkcs8_maybe_unchecked(key)?;
    let out = signing_key.sign(message);
    Ok(b64_encode(out))
}

// RSA
//
/// Only used internally when validating RSA, to map from our enum to the Ring param structs.
pub(crate) fn alg_to_rsa_parameters(alg: Algorithm) -> &'static signature::RsaParameters {
    match alg {
        Algorithm::RS256 => &signature::RSA_PKCS1_2048_8192_SHA256,
        Algorithm::RS384 => &signature::RSA_PKCS1_2048_8192_SHA384,
        Algorithm::RS512 => &signature::RSA_PKCS1_2048_8192_SHA512,
        Algorithm::PS256 => &signature::RSA_PSS_2048_8192_SHA256,
        Algorithm::PS384 => &signature::RSA_PSS_2048_8192_SHA384,
        Algorithm::PS512 => &signature::RSA_PSS_2048_8192_SHA512,
        _ => unreachable!("Tried to get RSA signature for a non-rsa algorithm"),
    }
}

/// Only used internally when signing with RSA, to map from our enum to the Ring signing structs.
pub(crate) fn alg_to_rsa_signing(alg: Algorithm) -> &'static dyn signature::RsaEncoding {
    match alg {
        Algorithm::RS256 => &signature::RSA_PKCS1_SHA256,
        Algorithm::RS384 => &signature::RSA_PKCS1_SHA384,
        Algorithm::RS512 => &signature::RSA_PKCS1_SHA512,
        Algorithm::PS256 => &signature::RSA_PSS_SHA256,
        Algorithm::PS384 => &signature::RSA_PSS_SHA384,
        Algorithm::PS512 => &signature::RSA_PSS_SHA512,
        _ => unreachable!("Tried to get RSA signature for a non-rsa algorithm"),
    }
}

/// The actual RSA signing + encoding
/// The key needs to be in PKCS8 format
/// Taken from Ring doc https://docs.rs/ring/latest/ring/signature/index.html
pub(crate) fn sign_rsa(
    alg: &'static dyn signature::RsaEncoding,
    key: &[u8],
    message: &[u8],
) -> Result<String> {
    let key_pair = signature::RsaKeyPair::from_der(key)
        .map_err(|e| ErrorKind::InvalidRsaKey(e.to_string()))?;

    let mut signature = vec![0; key_pair.public().modulus_len()];
    let rng = rand::SystemRandom::new();
    key_pair.sign(alg, &rng, message, &mut signature).map_err(|_| ErrorKind::RsaFailedSigning)?;

    Ok(b64_encode(signature))
}

/// Checks that a signature is valid based on the (n, e) RSA pubkey components
pub(crate) fn verify_from_components(
    alg: &'static signature::RsaParameters,
    signature: &str,
    message: &[u8],
    components: (&[u8], &[u8]),
) -> Result<bool> {
    let signature_bytes = b64_decode(signature)?;
    let pubkey = signature::RsaPublicKeyComponents { n: components.0, e: components.1 };
    let res = pubkey.verify(alg, message, &signature_bytes);
    Ok(res.is_ok())
}

// mod
//
// use crate::algorithms::Algorithm;
// use crate::decoding::{DecodingKey, DecodingKeyKind};
// use crate::encoding::EncodingKey;
// use crate::errors::Result;
// use crate::serialization::{b64_decode, b64_encode};

// pub(crate) mod ecdsa;
// pub(crate) mod eddsa;
// pub(crate) mod rsa;

/// The actual HS signing + encoding
/// Could be in its own file to match RSA/EC but it's 2 lines...
pub(crate) fn sign_hmac(alg: hmac::Algorithm, key: &[u8], message: &[u8]) -> String {
    let digest = hmac::sign(&hmac::Key::new(alg, key), message);
    b64_encode(digest)
}

/// Take the payload of a JWT, sign it using the algorithm given and return
/// the base64 url safe encoded of the result.
///
/// If you just want to encode a JWT, use `encode` instead.
pub fn sign(message: &[u8], key: &EncodingKey, algorithm: Algorithm) -> Result<String> {
    match algorithm {
        Algorithm::HS256 => Ok(sign_hmac(hmac::HMAC_SHA256, key.inner(), message)),
        Algorithm::HS384 => Ok(sign_hmac(hmac::HMAC_SHA384, key.inner(), message)),
        Algorithm::HS512 => Ok(sign_hmac(hmac::HMAC_SHA512, key.inner(), message)),

        Algorithm::ES256 | Algorithm::ES384 => {
            sign_ecdsa(alg_to_ec_signing(algorithm), key.inner(), message)
        }

        Algorithm::EdDSA => sign_eddsa(key.inner(), message),

        Algorithm::RS256
        | Algorithm::RS384
        | Algorithm::RS512
        | Algorithm::PS256
        | Algorithm::PS384
        | Algorithm::PS512 => sign_rsa(alg_to_rsa_signing(algorithm), key.inner(), message),
    }
}

/// See Ring docs for more details
fn verify_ring(
    alg: &'static dyn signature::VerificationAlgorithm,
    signature: &str,
    message: &[u8],
    key: &[u8],
) -> Result<bool> {
    let signature_bytes = b64_decode(signature)?;
    let public_key = signature::UnparsedPublicKey::new(alg, key);
    let res = public_key.verify(message, &signature_bytes);

    Ok(res.is_ok())
}

/// Compares the signature given with a re-computed signature for HMAC or using the public key
/// for RSA/EC.
///
/// If you just want to decode a JWT, use `decode` instead.
///
/// `signature` is the signature part of a jwt (text after the second '.')
///
/// `message` is base64(header) + "." + base64(claims)
pub fn verify(
    signature: &str,
    message: &[u8],
    key: &DecodingKey,
    algorithm: Algorithm,
) -> Result<bool> {
    match algorithm {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            // we just re-sign the message with the key and compare if they are equal
            let signed = sign(message, &EncodingKey::from_secret(key.as_bytes()), algorithm)?;
            Ok(verify_slices_are_equal(signature.as_ref(), signed.as_ref()).is_ok())
        }
        Algorithm::ES256 | Algorithm::ES384 => verify_ring(
            alg_to_ec_verification_ecdsa(algorithm),
            signature,
            message,
            key.as_bytes(),
        ),
        Algorithm::EdDSA => verify_ring(
            alg_to_ec_verification_eddsa(algorithm),
            signature,
            message,
            key.as_bytes(),
        ),
        Algorithm::RS256
        | Algorithm::RS384
        | Algorithm::RS512
        | Algorithm::PS256
        | Algorithm::PS384
        | Algorithm::PS512 => {
            let alg = alg_to_rsa_parameters(algorithm);
            match &key.kind {
                DecodingKeyKind::SecretOrDer(bytes) => verify_ring(alg, signature, message, bytes),
                DecodingKeyKind::RsaModulusExponent { n, e } => {
                    verify_from_components(alg, signature, message, (n, e))
                }
            }
        }
    }
}
