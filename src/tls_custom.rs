use std::future::Future;
use std::{io::{Read, Cursor, self}, net::SocketAddr};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncRead, AsyncWrite};

use tokio_rustls::TlsAcceptor;

use crate::server::{Certificates, Connection, Listener, SendableBufRead};

use rustls::{Certificate, PrivateKey, RootCertStore};
use indexmap::IndexSet;

fn err(message: impl Into<std::borrow::Cow<'static, str>>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, message.into())
}

/// Loads certificates from `reader`.
pub fn load_certs(reader: &mut SendableBufRead) -> io::Result<Vec<Certificate>> {
    let certs = rustls_pemfile::certs(reader.0.by_ref()).map_err(|_| err("invalid certificate"))?;
    Ok(certs.into_iter().map(Certificate).collect())
}

/// Load and decode the private key  from `reader`.
pub fn load_private_key(reader: &mut SendableBufRead) -> io::Result<PrivateKey> {
    // "rsa" (PKCS1) PEM files have a different first-line header than PKCS8
    // PEM files, use that to determine the parse function to use.
    let mut header = String::new();
    let private_keys_fn = loop {
        header.clear();
        if reader.0.read_line(&mut header)? == 0 {
            return Err(err("failed to find key header; supported formats are: RSA, PKCS8, SEC1"));
        }

        break match header.trim_end() {
            "-----BEGIN RSA PRIVATE KEY-----" => rustls_pemfile::rsa_private_keys,
            "-----BEGIN PRIVATE KEY-----" => rustls_pemfile::pkcs8_private_keys,
            "-----BEGIN EC PRIVATE KEY-----" => rustls_pemfile::ec_private_keys,
            _ => continue,
        };
    };

    let key = private_keys_fn(&mut Cursor::new(header).chain(reader.0.by_ref()))
        .map_err(|_| err("invalid key file"))
        .and_then(|mut keys| match keys.len() {
            0 => Err(err("no valid keys found; is the file malformed?")),
            1 => Ok(PrivateKey(keys.remove(0))),
            n => Err(err(format!("expected 1 key, found {}", n))),
        })?;

    // Ensure we can use the key.
    rustls::sign::any_supported_type(&key)
        .map_err(|_| err("key parsed but is unusable"))
        .map(|_| key)
}

/// Load and decode CA certificates from `reader`.
pub fn load_ca_certs(reader: &mut SendableBufRead) -> io::Result<RootCertStore> {
    let mut roots = rustls::RootCertStore::empty();
    for cert in load_certs(reader)? {
        roots.add(&cert).map_err(|e| err(format!("CA cert error: {}", e)))?;
    }

    Ok(roots)
}

// #[derive(Debug)]
// pub(crate) enum TLS_Config_Error {
//     IO(std::io::Error),
//     /// An Error parsing the Certificate
//     CertParseError,
//     /// Identity PEM is invalid
//     InvalidIdentityPem,
//     /// Identity PEM is missing a private key such as RSA, ECC or PKCS8
//     MissingPrivateKey,
//     /// Unknown private key format
//     UnknownPrivateKeyFormat,
//     /// An error from an empty key
//     EmptyKey,
//     /// An error from an invalid key
//     InvalidKey(TLS_Error),
// }
//
// impl std::fmt::Display for TLS_Config_Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             TLS_Config_Error::IO(err) => err.fmt(f),
//             TLS_Config_Error::CertParseError => write!(f, "certificate parse error"),
//             TLS_Config_Error::UnknownPrivateKeyFormat => write!(f, "unknown private key format"),
//             TLS_Config_Error::MissingPrivateKey => write!(f,"Identity PEM is missing a private key such as RSA, ECC or PKCS8"),
//             TLS_Config_Error::InvalidIdentityPem => write!(f, "identity PEM is invalid"),
//             TLS_Config_Error::EmptyKey => write!(f, "key contains no private key"),
//             TLS_Config_Error::InvalidKey(err) => write!(f, "key contains an invalid key, {}", err),
//         }
//     }
// }
///
/// A supported TLS cipher suite.
#[allow(non_camel_case_types)]
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, Deserialize, Serialize)]
#[cfg_attr(nightly, doc(cfg(feature = "tls")))]
#[non_exhaustive]
pub enum CipherSuite {
    /// The TLS 1.3 `TLS_CHACHA20_POLY1305_SHA256` cipher suite.
    TLS_CHACHA20_POLY1305_SHA256,
    /// The TLS 1.3 `TLS_AES_256_GCM_SHA384` cipher suite.
    TLS_AES_256_GCM_SHA384,
    /// The TLS 1.3 `TLS_AES_128_GCM_SHA256` cipher suite.
    TLS_AES_128_GCM_SHA256,

    /// The TLS 1.2 `TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256` cipher suite.
    TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
    /// The TLS 1.2 `TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256` cipher suite.
    TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
    /// The TLS 1.2 `TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384` cipher suite.
    TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
    /// The TLS 1.2 `TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256` cipher suite.
    TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
    /// The TLS 1.2 `TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384` cipher suite.
    TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    /// The TLS 1.2 `TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256` cipher suite.
    TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
}

impl CipherSuite {
    /// The default set and order of cipher suites. These are all of the
    /// variants in [`CipherSuite`] in their declaration order.
    pub const DEFAULT_SET: [CipherSuite; 9] = [
        // TLS v1.3 suites...
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_AES_128_GCM_SHA256,

        // TLS v1.2 suites...
        CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    ];

    /// The default set and order of cipher suites. These are the TLS 1.3
    /// variants in [`CipherSuite`] in their declaration order.
    pub const TLS_V13_SET: [CipherSuite; 3] = [
        CipherSuite::TLS_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_AES_256_GCM_SHA384,
        CipherSuite::TLS_AES_128_GCM_SHA256,
    ];

    /// The default set and order of cipher suites. These are the TLS 1.2
    /// variants in [`CipherSuite`] in their declaration order.
    pub const TLS_V12_SET: [CipherSuite; 6] = [
        CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
        CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
    ];

    /// Used as the `serde` default for `ciphers`.
    fn default_set() -> IndexSet<Self> {
        Self::DEFAULT_SET.iter().copied().collect()
    }
}

pub struct TLS_Config {
    // pub cert_chain: Box<dyn io::BufRead>,
    pub cert_chain: SendableBufRead,
    // pub private_key: Box<dyn io::BufRead>,
    pub private_key: SendableBufRead,
    pub ciphersuites: Vec<rustls::SupportedCipherSuite>,
    pub ciphers: IndexSet<CipherSuite>,
    pub prefer_server_order: bool,
    // pub ca_certs: Option<Box<dyn io::BufRead>>,
    pub ca_certs: Option<SendableBufRead>,
    pub mandatory_mtls: bool,
    prefer_server_cipher_order: bool,
}

pub fn ciphers(ciphers: &IndexSet<CipherSuite>) -> impl Iterator<Item = CipherSuite> + '_ {
    ciphers.iter().copied()
}

impl TLS_Config {
    pub fn new(key: &[u8], cert: &[u8], ocsp: &[u8]) -> Self {
        let ciphers = CipherSuite::default_set();
        TLS_Config {
            private_key: SendableBufRead(Box::new(Cursor::new(Vec::from(key)))),
            cert_chain: SendableBufRead(Box::new(Cursor::new(Vec::from(cert)))),
            // private_key: Vec::from(key),
            // cert_chain: Vec::from(cert),

            // auth: TLS_Client_Auth::Off,
            // ocsp: Vec::from(ocsp),
            prefer_server_cipher_order: false,
            mandatory_mtls: false,
            prefer_server_order: false,
            ciphersuites: Self::rustls_ciphers(&ciphers).collect(), 
            ciphers, 

            ca_certs: None,
            // prefer_server_order: 
        }
    }

    fn rustls_ciphers(ciphers_data: &IndexSet<CipherSuite>) -> impl Iterator<Item = rustls::SupportedCipherSuite> + '_ {
        use rustls::cipher_suite;

        ciphers(ciphers_data).map(|ciphersuite| match ciphersuite {
            CipherSuite::TLS_CHACHA20_POLY1305_SHA256 =>
                cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
            CipherSuite::TLS_AES_256_GCM_SHA384 =>
                cipher_suite::TLS13_AES_256_GCM_SHA384,
            CipherSuite::TLS_AES_128_GCM_SHA256 =>
                cipher_suite::TLS13_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256 =>
                cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
            CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256 =>
                cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384 =>
                cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
            CipherSuite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256 =>
                cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384 =>
                cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
            CipherSuite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256 =>
                cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        })
    }
}

pub struct TLS_Listener {
    pub listener: TcpListener,
    pub acceptor: TlsAcceptor,
}

impl TLS_Listener {
    pub async fn bind(addr: SocketAddr, mut c: TLS_Config) -> io::Result<TLS_Listener>
    // pub async fn bind<R>(addr: SocketAddr, mut c: TLS_Config) -> io::Result<TLS_Listener>
        // where R: io::BufRead
    {
        use rustls::server::{AllowAnyAuthenticatedClient, AllowAnyAnonymousOrAuthenticatedClient};
        use rustls::server::{NoClientAuth, ServerSessionMemoryCache, ServerConfig};

        let cert_chain = load_certs(&mut c.cert_chain)
            .map_err(|e| io::Error::new(e.kind(), format!("bad TLS cert chain: {}", e)))?;

        let key = load_private_key(&mut c.private_key)
            .map_err(|e| io::Error::new(e.kind(), format!("bad TLS private key: {}", e)))?;

        let client_auth = match c.ca_certs {
            Some(ref mut ca_certs) => match load_ca_certs(ca_certs) {
                Ok(ca) if c.mandatory_mtls => AllowAnyAuthenticatedClient::new(ca).boxed(),
                Ok(ca) => AllowAnyAnonymousOrAuthenticatedClient::new(ca).boxed(),
                Err(e) => return Err(io::Error::new(e.kind(), format!("bad CA cert(s): {}", e))),
            },
            None => NoClientAuth::boxed(),
        };

        // println!("cert: {:?}", cert_chain);
        // println!("key: {:?}", key);

        let mut tls_config = ServerConfig::builder()
            .with_cipher_suites(&c.ciphersuites)
            .with_safe_default_kx_groups()
            .with_safe_default_protocol_versions()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?
            .with_client_cert_verifier(client_auth)
            .with_single_cert(cert_chain, key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?;

        // let mut tls_config = ServerConfig::builder()
        //     // .with_cipher_suites(&c.ciphersuites)
        //     .with_safe_defaults()
        //     // .with_safe_default_protocol_versions()
        //     .with_no_client_auth()
        //     // .with_single_cert(Vg, key_der);
        //     // .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?
        //     // .with_client_cert_verifier(client_auth)
        //     .with_single_cert(cert_chain, key)
        //     .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS config: {}", e)))?;

        tls_config.ignore_client_order = c.prefer_server_order;

        tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
        // if cfg!(feature = "http2") {
        //     tls_config.alpn_protocols.insert(0, b"h2".to_vec());
        // }

        tls_config.session_storage = ServerSessionMemoryCache::new(1024);
        tls_config.ticketer = rustls::Ticketer::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("bad TLS ticketer: {}", e)))?;

        let listener = TcpListener::bind(addr).await?;
        let acceptor = TlsAcceptor::from(Arc::new(tls_config));
        Ok(TLS_Listener { listener, acceptor })
    }
}

impl Listener for TLS_Listener {
    type Connection = TLS_Stream;

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<io::Result<Self::Connection>> {
        match futures::ready!(self.listener.poll_accept(cx)) {
            Ok((io, addr)) => Poll::Ready(Ok(TLS_Stream {
                remote: addr,
                state: TLS_State::Handshaking(self.acceptor.accept(io)),
                // These are empty and filled in after handshake is complete.
                certs: Certificates::default(),
            })),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl Connection for TLS_Stream {
    fn peer_address(&self) -> Option<SocketAddr> {
        Some(self.remote)
    }

    fn enable_nodelay(&self) -> io::Result<()> {
        // If `Handshaking` is `None`, it either failed, so we returned an `Err`
        // from `poll_accept()` and there's no connection to enable `NODELAY`
        // on, or it succeeded, so we're in the `Streaming` stage and we have
        // infallible access to the connection.
        match &self.state {
            TLS_State::Handshaking(accept) => match accept.get_ref() {
                None => Ok(()),
                Some(s) => s.enable_nodelay(),
            },
            TLS_State::Streaming(stream) => stream.get_ref().0.enable_nodelay()
        }
    }

    fn peer_certificates(&self) -> Option<Certificates> {
        Some(self.certs.clone())
    }
}

enum TLS_State {
    Handshaking(tokio_rustls::Accept<TcpStream>),
    Streaming(tokio_rustls::server::TlsStream<TcpStream>),
}

pub struct TLS_Stream {
    remote: SocketAddr,
    state: TLS_State,
    certs: Certificates,
}

impl TLS_Stream {
    fn poll_accept_then<F, T>(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut f: F
    ) -> Poll<io::Result<T>>
        where F: FnMut(&mut tokio_rustls::server::TlsStream<TcpStream>, &mut Context<'_>) -> Poll<io::Result<T>>
    {
        loop {
            match self.state {
                TLS_State::Handshaking(ref mut accept) => {
                    match futures::ready!(Pin::new(accept).poll(cx)) {
                        Ok(stream) => {
                            if let Some(cert_chain) = stream.get_ref().1.peer_certificates() {
                                self.certs.set(cert_chain.to_vec());
                            }

                            self.state = TLS_State::Streaming(stream);
                        }
                        Err(e) => {
                            // log::warn!("tls handshake with {} failed: {}", self.remote, e);
                            println!("tls handshake with {} failed: {}", self.remote, e);
                            return Poll::Ready(Err(e));
                        }
                    }
                },
                TLS_State::Streaming(ref mut stream) => return f(stream, cx),
            }
        }
    }
}

impl AsyncRead for TLS_Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.poll_accept_then(cx, |stream, cx| Pin::new(stream).poll_read(cx, buf))
    }
}

impl AsyncWrite for TLS_Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.poll_accept_then(cx, |stream, cx| Pin::new(stream).poll_write(cx, buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.state {
            TLS_State::Handshaking(accept) => match accept.get_mut() {
                Some(io) => Pin::new(io).poll_flush(cx),
                None => Poll::Ready(Ok(())),
            }
            TLS_State::Streaming(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match &mut self.state {
            TLS_State::Handshaking(accept) => match accept.get_mut() {
                Some(io) => Pin::new(io).poll_shutdown(cx),
                None => Poll::Ready(Ok(())),
            }
            TLS_State::Streaming(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
// #[derive(Debug, Clone)]
// enum TLS_Client_Auth {
//     /// No client auth.
//     Off,
//     /// Allow any anonymous or authenticated client.
//     Optional(Box<dyn Read + Send + Sync>),
//     /// Allow any authenticated client.
//     Required(Box<dyn Read + Send + Sync>),
// }

// #[derive(Clone)]
// pub struct TLS_Config {
//     // key : Box<dyn Read + Send + Sync>,// String,
//     // cert: Box<dyn Read + Send + Sync>, //String,
//     key : Vec<u8>,// String,
//     cert: Vec<u8>, //String,
//     // auth: TLS_Client_Auth,
//     ocsp: Vec<u8>,
// }

//
//     pub(crate) fn build(mut self) -> Result<ServerConfig, TLS_Config_Error> {
//         let mut cert_rdr = BufReader::new(self.cert);
//         let cert = rustls_pemfile::certs(&mut cert_rdr)
//             .collect::<Result<Vec<_>, _>>()
//             .map_err(|_e| TLS_Config_Error::CertParseError)?;
//
//         let mut key_vec = Vec::new();
//         self.key
//             .read_to_end(&mut key_vec)
//             .map_err(TLS_Config_Error::IO)?;
//
//         if key_vec.is_empty() {
//             return Err(TLS_Config_Error::EmptyKey);
//         }
//
//         let mut key_opt = None;
//         let mut key_cur = std::io::Cursor::new(key_vec);
//         for item in rustls_pemfile::read_all(&mut key_cur)
//             .collect::<Result<Vec<_>, _>>()
//             .map_err(|_e| TLS_Config_Error::InvalidIdentityPem)?
//         {
//             match item {
//                 rustls_pemfile::Item::Pkcs1Key(k) => key_opt = Some(k.into()),
//                 rustls_pemfile::Item::Pkcs8Key(k) => key_opt = Some(k.into()),
//                 rustls_pemfile::Item::Sec1Key(k) => key_opt = Some(k.into()),
//                 _ => return Err(TLS_Config_Error::UnknownPrivateKeyFormat),
//             }
//         }
//         let key = match key_opt {
//             Some(v) => v,
//             _ => return Err(TLS_Config_Error::MissingPrivateKey),
//         };
//
//         fn read_trust_anchor(
//             trust_anchor: Box<dyn Read + Send + Sync>,
//         ) -> Result<RootCertStore, TLS_Config_Error> {
//             let trust_anchors = {
//                 let mut reader = BufReader::new(trust_anchor);
//                 rustls_pemfile::certs(&mut reader)
//                     .collect::<Result<Vec<_>, _>>()
//                     .map_err(TLS_Config_Error::IO)?
//             };
//
//             let mut store = RootCertStore::empty();
//             let (added, _skipped) = store.add_parsable_certificates(trust_anchors);
//             if added == 0 {
//                 return Err(TLS_Config_Error::CertParseError);
//             }
//
//             Ok(store)
//         }
//
//         let config = {
//             let builder = ServerConfig::builder();
//             let mut config = match self.auth {
//                 TLS_Client_Auth::Off => builder.with_no_client_auth(),
//                 TLS_Client_Auth::Optional(trust_anchor) => {
//                     let verifier =
//                         WebPkiClientVerifier::builder(read_trust_anchor(trust_anchor)?.into())
//                             .allow_unauthenticated()
//                             .build()
//                             .map_err(|_| TLS_Config_Error::CertParseError)?;
//                     builder.with_client_cert_verifier(verifier)
//                 }
//                 TLS_Client_Auth::Required(trust_anchor) => {
//                     let verifier =
//                         WebPkiClientVerifier::builder(read_trust_anchor(trust_anchor)?.into())
//                             .build()
//                             .map_err(|_| TLS_Config_Error::CertParseError)?;
//                     builder.with_client_cert_verifier(verifier)
//                 }
//             }
//             .with_single_cert_with_ocsp(cert, key, self.ocsp)
//             .map_err(TLS_Config_Error::InvalidKey)?;
//             config.alpn_protocols = vec!["h2".into(), "http/1.1".into()];
//             config
//         };
//
//         Ok(config)
//     }
// }

// impl std::error::Error for TLS_Config_Error {}
