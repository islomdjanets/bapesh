use std::fs::File;
use std::io::{BufReader};
use std::{str};

use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::ServerConfig;
use rustls_pemfile::{certs, rsa_private_keys};

use axum_server::tls_rustls::RustlsConfig;

// Load TLS certificates and private key
pub fn load_config(
    cert_path: &str,
    key_path: &str,
// ) -> Result<ServerConfig, Box<dyn std::error::Error>> {
) -> Result<RustlsConfig, std::io::Error> {
    // Load certificates
    let cert_file = File::open(cert_path)?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer> = certs(&mut cert_reader)
        .map(|result| result.map(|der| CertificateDer::from(der.to_vec())))
        .collect::<Result<_, _>>()?;

    // Load private key
    let key_file = File::open(key_path)?;
    let mut key_reader = BufReader::new(key_file);
    let mut keys: Vec<PrivateKeyDer> = rsa_private_keys(&mut key_reader)
        .map(|result| result.map(|der| PrivateKeyDer::Pkcs1(der)))
        .collect::<Result<_, _>>()?;
    let key = keys.remove(0); // Assume one key

    // Configure rustls
    let mut config = ServerConfig::builder()
        // .with_safe_defaults()
        .with_no_client_auth()
        // .with_protocol_versions(&[rustls::ProtocolVersion::TLSv1_3, rustls::ProtocolVersion::TLSv1_2])
        // .alpn_protocols(vec![b"h2".to_vec(), b"http/1.1".to_vec()])
        .with_single_cert(certs, key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    // config.versions = vec![rustls::ProtocolVersion::TLSv1_3, rustls::ProtocolVersion::TLSv1_2];

    // Create RustlsConfig from ServerConfig
    Ok(RustlsConfig::from_config(std::sync::Arc::new(config)))
    // Ok(config)
}

pub fn load_config_default() -> Result<RustlsConfig, std::io::Error> {
    load_config("./tls/cert.pem", "./tls/key.pem")
}