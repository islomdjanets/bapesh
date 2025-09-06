use crate::{db::Database, tls::{TLS_Config, TLS_Listener}};
use core::fmt;
use std::{
    any::{Any, TypeId}, collections::HashMap, hash::{BuildHasherDefault, Hasher}, io::BufRead, net::{IpAddr, SocketAddr, ToSocketAddrs}, ops::Deref, pin::Pin, str, sync::{Arc, Mutex}, task::{Context, Poll}, time::Duration
};
use rustls::{OwnedTrustAnchor, RootCertStore};
use tokio::{time};

use anyhow::anyhow;
use tokio::{net::{TcpListener, TcpStream}, signal};

use tokio_rustls::{rustls::ServerConfig};
use tokio_util::sync::CancellationToken;

use tokio::io::{AsyncWriteExt, AsyncRead, AsyncWrite};

use crate::{cors, driver::Driver, env, handshake::{Method, Request, Response}};

use state::InitCell;

pub type Handler = fn(&Request, &mut Resources) -> Response;
pub type Check = fn( &Request ) -> bool;


pub struct SendableBufRead(pub Box<dyn BufRead + Send>);

unsafe impl Send for SendableBufRead {}

// pub type Handler = fn(&Request, &mut Resources) -> dyn Responder;

// pub trait Handler {
//     fn handle() {
//
//     }
// }
// pub type Handler_Fn = fn(&Request, &mut Resources) -> Response;
// pub trait Check {
//     fn check( &self, request: &Request ) -> bool;
// }
//
// pub type Check_Fn = fn( &Request ) -> bool;
//
// impl Check for Check_Fn {
//     fn check( &self, request: &Request ) -> bool {
//         &self( request ) 
//     } 
// }

// pub type Routes = Arc<Mutex<HashMap<Method, Vec<Route>>>>;
pub type Routes = Arc<Mutex<Vec<Route>>>;
pub type Middlewares = Arc<Mutex<Vec<Middleware>>>;
pub type Services = Arc<Mutex<Vec<Arc<dyn Service>>>>;

// unsafe impl Send for Routes {}

#[derive(Debug, Default)]
struct NoOpHasher(u64);

impl Hasher for NoOpHasher {
    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!("This NoOpHasher can only handle u64s")
    }

    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Route {
    method: Method,
    uri: Check,
    handler: Handler
}
unsafe impl Send for Route {}

pub type Middleware = fn(&mut Response, &mut Resources);

pub struct Data<T: ?Sized>(Arc<T>);
impl<T> Data<T> {
    /// Create new `Data` instance.
    pub fn new(state: T) -> Data<T> {
        Data(Arc::new(state))
    }
}

impl<T: ?Sized> Data<T> {
    /// Returns reference to inner `T`.
    pub fn get_ref(&self) -> &T {
        self.0.as_ref()
    }

    /// Unwraps to the internal `Arc<T>`
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T: ?Sized> Deref for Data<T> {
    type Target = Arc<T>;

    fn deref(&self) -> &Arc<T> {
        &self.0
    }
}

impl<T: ?Sized> Clone for Data<T> {
    fn clone(&self) -> Data<T> {
        Data(Arc::clone(&self.0))
    }
}

impl<T: ?Sized> From<Arc<T>> for Data<T> {
    fn from(arc: Arc<T>) -> Self {
        Data(arc)
    }
}

impl<T: Default> Default for Data<T> {
    fn default() -> Self {
        Data::new(T::default())
    }
}

// impl<T> Serialize for Data<T>
// where
//     T: Serialize,
// {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer,
//     {
//         self.0.serialize(serializer)
//     }
// }
// impl<'de, T> de::Deserialize<'de> for Data<T>
// where
//     T: de::Deserialize<'de>,
// {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: de::Deserializer<'de>,
//     {
//         Ok(Data::new(T::deserialize(deserializer)?))
//     }
// }
//
#[derive(Default)]
pub struct Resources {
    /// Use AHasher with a std HashMap with for faster lookups on the small `TypeId` keys.
    map: HashMap<TypeId, Box<dyn Any>, BuildHasherDefault<NoOpHasher>>,
}
unsafe impl Send for Resources {}

impl Resources {
    #[inline]
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
        }
    }

    pub fn insert<T: 'static>(&mut self, val: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(downcast_owned)
    }

    pub fn contains<T: 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut())
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.map.remove(&TypeId::of::<T>()).and_then(downcast_owned)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn extend(&mut self, other: Resources) {
        self.map.extend(other.map);
    }
}

impl fmt::Debug for Resources {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extensions").finish()
    }
}

fn downcast_owned<T: 'static>(boxed: Box<dyn Any>) -> Option<T> {
    boxed.downcast().ok().map(|boxed| *boxed)
}

// pub struct Service { // must be trait
//     uri: Check,
//     handler: fn( &mut TcpStream, Arc<Mutex<Resources>> ),
// }
pub trait Service: Sync + Send { // must be trait
    fn uri(&self, request: &Request) -> bool;
    fn handler(&self, stream: &mut TcpStream, resources: Arc<Mutex<Resources>>);
    // async fn serve(&self);
}

pub use rustls::Certificate as CertificateData;

/// A thin wrapper over raw, DER-encoded X.509 client certificate data.
// #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
// pub struct CertificateData(pub Vec<u8>);

/// A collection of raw certificate data.
#[derive(Clone, Default)]
pub struct Certificates(Arc<InitCell<Vec<CertificateData>>>);

impl From<Vec<CertificateData>> for Certificates {
    fn from(value: Vec<CertificateData>) -> Self {
        Certificates(Arc::new(value.into()))
    }
}

impl Certificates {
    /// Set the the raw certificate chain data. Only the first call actually
    /// sets the data; the remaining do nothing.
    pub(crate) fn set(&self, data: Vec<CertificateData>) {
        self.0.set(data);
    }

    /// Returns the raw certificate chain data, if any is available.
    pub fn chain_data(&self) -> Option<&[CertificateData]> {
        self.0.try_get().map(|v| v.as_slice())
    }
}

// TODO.async: 'Listener' and 'Connection' provide common enough functionality
// that they could be introduced in upstream libraries.
/// A 'Listener' yields incoming connections
pub trait Listener {
    /// The connection type returned by this listener.
    type Connection: Connection;

    /// Return the actual address this listener bound to.
    fn local_addr(&self) -> Option<SocketAddr>;

    /// Try to accept an incoming Connection if ready. This should only return
    /// an `Err` when a fatal problem occurs as Hyper kills the server on `Err`.
    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<std::io::Result<Self::Connection>>;
}

/// A 'Connection' represents an open connection to a client
pub trait Connection: AsyncRead + AsyncWrite {
    /// The remote address, i.e. the client's socket address, if it is known.
    fn peer_address(&self) -> Option<SocketAddr>;

    /// Requests that the connection not delay reading or writing data as much
    /// as possible. For connections backed by TCP, this corresponds to setting
    /// `TCP_NODELAY`.
    fn enable_nodelay(&self) -> std::io::Result<()>;

    /// DER-encoded X.509 certificate chain presented by the client, if any.
    ///
    /// The certificate order must be as it appears in the TLS protocol: the
    /// first certificate relates to the peer, the second certifies the first,
    /// the third certifies the second, and so on.
    ///
    /// Defaults to an empty vector to indicate that no certificates were
    /// presented.
    fn peer_certificates(&self) -> Option<Certificates> { None }
}

impl Listener for TcpListener {
    type Connection = TcpStream;

    #[inline]
    fn local_addr(&self) -> Option<SocketAddr> {
        self.local_addr().ok()
    }

    #[inline]
    fn poll_accept(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>
    ) -> Poll<std::io::Result<Self::Connection>> {
        (*self).poll_accept(cx).map_ok(|(stream, _addr)| stream)
    }
}

impl Connection for TcpStream {
    #[inline]
    fn peer_address(&self) -> Option<SocketAddr> {
        self.peer_addr().ok()
    }

    #[inline]
    fn enable_nodelay(&self) -> std::io::Result<()> {
        self.set_nodelay(true)
    }
}

// #[derive(Clone)]
pub struct Server {
    host: String,
    port: u16,

    // tls_config: Option<TLS_Config>, 
    tls: bool,

    routes: Routes,
    middlewares: Middlewares,
    pub resources: Arc<Mutex<Resources>>,
    services: Services,
}

fn is_ping(request: &Request) -> bool {
    request.uri == "/ping"
}

fn pong(_: &Request, _: &mut Resources) -> Response {
    Response::new()
}

fn is_from_postman(request: &Request) -> bool {
    // if request.headers.contains_key("Bapesh-Token") {
    //     return false;
    // }
    request.headers.contains_key("Postman-Token")
}

fn block(_: &Request, _: &mut Resources) -> Response {
    println!("access denied");
    Response::not_ok("access denied")
}

impl Server {
    pub fn new() -> Self {
        env::ok();

        let mut server = Server {
            host: "127.0.0.1".into(),
            port: 7878,

            tls: false,

            routes: Arc::new(Mutex::new(Vec::new())),
            middlewares: Arc::new(Mutex::new(Vec::new())),
            resources: Arc::new(Mutex::new(Resources::new())),
            services: Arc::new(Mutex::new(Vec::new())),
        };

        // server.get(is_from_postman, block);
        server.get(is_ping, pong);

        server
    }

    pub fn tick(&mut self) {

    }

    pub fn tls(&mut self) -> &mut Self {

        // let server_config = config.build();
        // if let Ok(server_config) = server_config {
            // println!("{:?}", server_config);
            self.tls = true;
        // }
            
        self
    }

    pub fn cors(&mut self, options: cors::Cors) -> &mut Self { 
        self.add_resourse(options);
        self.add_middleware(cors::middleware);
        self.add_route( Route {
            method: Method::OPTIONS,
            uri: cors::is_preflight,
            handler: cors::handle_preflight
        });
        self
    }

    // pub fn ws( &mut self, options: ws::WebSocket ) -> &mut Self { // remove!!!
    //
    //     self.add_route( Route {
    //         method: Method::GET,
    //         uri: options.uri,
    //         handler: ws::update_to_websocket
    //     });
    //
    //     self.service( Service {
    //         uri: options.uri,
    //         handler: options.handler,
    //     });
    //
    //     self.add_resourse(options);
    //     self
    // }

    pub fn service(&mut self, service: Arc<dyn Service>) -> &mut Self {
        self.services.lock().unwrap().push(service);
        println!("added new service");
        self
    }

    pub fn add_middleware(&mut self, middleware: Middleware ) -> &mut Self {
        self.middlewares.lock().unwrap().push(middleware);
        self
    }

    pub fn add_resourse<U: 'static>(&mut self, resourse: U) -> &mut Self {
        // let resourse = Data::new(resourse);
        self.resources.lock().unwrap().insert(resourse);
        self
    }

    pub fn bind(&mut self, host: &str, port: u16) -> &mut Self {
        
        self.host = host.into();
        self.port = port;

        self
    }

    pub fn host(&mut self, host: &str) -> &mut Self {
        self.host = host.into();
        self
    }

    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    fn add_route(&mut self, route: Route) { 
        let mut routes = self.routes.lock().unwrap();

        // match routes.get_mut(&method) {
        //     Some(routes) => routes.push(route),
        //     None => {
        //         routes.insert(method.clone(), Vec::new());
        //         routes.get_mut(&method).unwrap().push(route);
        //     },
        // };
        routes.push(route);
    }

    pub fn get(&mut self, uri: Check, handler: Handler ) -> &mut Self {
        self.add_route(Route {
            method: Method::GET,
            uri,
            handler,
        });

        self
    }

    pub fn post(&mut self, uri: Check, handler: Handler ) -> &mut Self {
        self.add_route(Route {
            method: Method::POST,
            uri,
            handler,
        });

        self
    }

    pub fn delete(&mut self, uri: Check, handler: Handler ) -> &mut Self {
        self.add_route(Route {
            method: Method::DELETE,
            uri,
            handler,
        });

        self
    }

    pub fn put(&mut self, uri: Check, handler: Handler ) -> &mut Self {
        self.add_route(Route {
            method: Method::PUT,
            uri,
            handler,
        });

        self
    }

    pub fn options(&mut self, uri: Check, handler: Handler ) -> &mut Self {
        self.add_route(Route {
            method: Method::OPTIONS,
            uri,
            handler,
        });

        self
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

// fn create_listener(listener_type: &str) -> Box<dyn Listener> {
//     match listener_type {
//         "tls" => Box::new(TLS_Listener),
//         "tcp" => Box::new(TcpListener),
//         _ => panic!("Unknown listener type"),
//     }
// }

// async fn get_listener<L: Listener>(server: &Server) -> L {
async fn get_listener(server: &Server) -> TLS_Listener {
    let addr = format!("{}:{}", server.host, server.port);
    // match server.tls {
    //     true => {
            // let key_path = "tls/key.pem";
            // let cert_path = "tls/cert.pem";
            let key_path = "tls/localhost.key";
            let cert_path = "tls/localhost.crt";

            let key = Driver::read(key_path);
            let cert = Driver::read(cert_path);

            if key.is_err() {
                println!("key is not valid or doesn't exists");
            }
            if cert.is_err() {
                println!("cert is not valid or doesn't exists");
            }

            let key = key.unwrap();
            let cert = cert.unwrap();

            let config = TLS_Config::new(
                key.as_slice(),
                cert.as_slice(),
                Vec::new().as_slice(),
            );
            let listener = TLS_Listener::bind(
                addr.parse::<SocketAddr>().unwrap(), 
                config
            ).await.unwrap(); 

            println!("securly listening for connections on host: {} port: {}", server.host, server.port);
            // return Box::new(listener) as L;
            listener
        // },
        // false => {
        //     let listener = TcpListener::bind(addr).await.unwrap();
        //     println!("listening for connections on host: {} port: {}", server.host, server.port);
        //     return Box::new(listener) as L;
        // }
    // };

}

pub fn get_ip() -> Option<IpAddr> {
    match get_if_addrs::get_if_addrs() {
        Ok(interfaces) => {
            for iface in interfaces {
                if iface.is_loopback() {
                    continue;
                }
                // Check if it's an IPv4 address (you can also check for IPv6 if needed)
                let ip = iface.ip();
                if ip.is_ipv4() {
                    println!("Interface: {} IP: {}", iface.name, ip);
                    return Some(ip);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
    None
}

pub async fn run(server: Server) {

    let server = Arc::new(server);

    // let saver = tokio::spawn(async move {
    //     // let resources = server.resources.lock().unwrap();
    //     // let db = resources.get_mut::<Database>().unwrap();
    //
    //     loop {
    //         interval.tick().await;
    //         println!("hello");
    //         // server.tick();
    //         // db.save();
    //     }
    // });

    let cancel_token = CancellationToken::new();
    tokio::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            if let Ok(()) = signal::ctrl_c().await {
                // info!("received Ctrl-C, shutting down");
                cancel_token.cancel();
            }
        }
    });

    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
            |ta| {
                OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                )  
            }
    ));

    // let listener: dyn Listener<Connection = dyn Connection> = match server.tls {
    // let listener: Box<dyn Listener<Connection = dyn self::Connection>> = match server.tls {
    let tls = get_listener(&server).await;

    let mut interval = time::interval(Duration::from_secs(30));
    let mut tasks = Vec::new();
    // let cancel_token = cancel_token.clone();
    loop {

        tokio::select! {
            _ = interval.tick() => {
                let server = Arc::clone(&server);//.clone();
                let mut resources = server.resources.lock().unwrap();
                if let Some(db) = resources.get_mut::<Database>() {
                    db.save();
                    println!("save");
                }
            },
            Ok((stream, addr)) = tls.listener.accept() => {
                // let value = tls.poll_accept();
                let mut stream = tls.acceptor.accept(stream).await.unwrap();
                // if stream.is_err() {
                //     return;
                // }
                // let mut stream = stream.unwrap();
            // let (mut stream, addr) = listener.accept().await.unwrap();
                // let mut stream = BufStream::new(stream);

                let request = Request::new(&mut stream).await;
                if request.is_none() {
                    continue;
                    // return;
                }
                let request = request.unwrap();
                let close_connection = request.headers.get("Connection") == Some(&"close".to_string());
                if close_connection {
                    println!("close connection");
                    break;
                    // return;
                }

                let value = Arc::clone(&server);//.clone();
                let client_task = tokio::spawn(async move {
                // thread_pool.execute(move || { 
                    let response = handle_connection(
                        request,
                        value,
                    ).await;

                    if response.is_err() {
                        return;
                    }

                    let response = response.unwrap();

                    let content = response.get();
                    let data = content.as_bytes();
                    stream.write_all(data).await.unwrap();
                    // tokio::io::copy(&mut data, &mut stream).await?; // ????

                    if !response.body.is_empty() {
                        // stream.write_all(&response.body).await?;
                        let data = response.body.as_slice();
                        // let mut data = content.as_bytes();
                        stream.write_all(data).await.unwrap();
                        // tokio::io::copy(&mut data, &mut stream).await?; // ????
                        // if result.is_ok() {
                        //     result.unwrap()
                        // }
                    }
                    // tokio::io::copy(&mut self.data, stream).await?; // ????
                    stream.flush().await.unwrap();
                    // stream.shutdown().await?;
            // return; 
                });
                tasks.push(client_task);
            },
            _ = cancel_token.cancelled() => {
                println!("cancelled");
                break;
            }
        }


        // let (stream, addr) = listener.accept().await.unwrap();
        // let mut stream = BufStream::new(stream);
        //
        // let value = server.clone();
        // // tokio::spawn(async move {
        //     // thread_pool.execute(move || { 
        //     handle_connection(
        //         stream,
        //         Arc::clone(&value.routes),
        //         Arc::clone(&value.resources),
        //         Arc::clone(&value.middlewares),
        //         // addr,
        //     ).await.unwrap();
        //     // });
        // // });
    }
        println!("hello");
            futures::future::join_all(tasks).await;
    // return;
    // listener.incoming().for_each(|stream| {
    //     let value = server.clone();
    //     let mut stream = BufStream::new(stream.unwrap());
    //     // thread_pool.execute(move || { 
    //         handle_connection(
    //             stream,
    //             Arc::clone( &value.routes ),
    //             Arc::clone( &value.resources ),
    //             Arc::clone( &value.middlewares ),
    //         );
        // });
        // handle_connection(
        //     stream.unwrap(),
        //     Arc::clone( &self.routes ),
        //     Arc::clone( &self.resources ),
        //     Arc::clone( &self.middlewares ),
        // );
    // });
}

async fn handle_connection<'a>(
    // mut stream: impl AsyncBufRead + AsyncWrite + Unpin,
    request: Request,
    // routes: Routes,
    // resources: Arc<Mutex<Resources>>,
    // middlewares: Middlewares,
    server: Arc<Server>,
    ) -> anyhow::Result<Response> {

    // let request = Request::new(&mut stream).await?;
    // let close_connection = request.headers.get("Connection") == Some(&"close".to_string());
    //
    // println!("{}", request);
    // if request.is_err() {
    //     return;
    // }
    //
    // let request = request.unwrap();
    // println!("{}", request);

    // if let Some(routes) = routes.lock().unwrap().get(&request.method) {
    let routes = server.routes.lock().unwrap();
    for route in routes.iter() {
        if route.method == request.method && (route.uri)(&request) {

            let mut resources = server.resources.lock().unwrap(); 
            let mut response = (route.handler)(&request, &mut resources);

            let middlewares = server.middlewares.lock().unwrap();
            for middleware in middlewares.iter() {
                (middleware)(&mut response, &mut resources);
            }

            return Ok(response);
        } 
    }

    Err(anyhow!("error"))

    // Ok()

    // let services = services.lock().unwrap();
    // for service in services.iter() {
    //     if (service.uri)(&request) {
    //         (service.handle)(&mut stream, resources);
    //         return;
    //     }
    // }
}


pub async fn run_not_tls(server: Server) {
    let server = Arc::new(server);

    let cancel_token = CancellationToken::new();
    tokio::spawn({
        let cancel_token = cancel_token.clone();
        async move {
            if let Ok(()) = signal::ctrl_c().await {
                // info!("received Ctrl-C, shutting down");
                cancel_token.cancel();
            }
        }
    });

    if let Some(ip) = get_ip() {
        let ip = ip.to_string();
        println!("-> Local   : {}:{}", server.host, server.port);
        println!("-> Network : {}:{}", ip, server.port);
    }

    // let addr = format!("{}:{}", server.host, server.port);
    let addr = format!("{}:{}", "0.0.0.0", server.port);
    let listener = TcpListener::bind(addr).await.unwrap();

    let mut interval = time::interval(Duration::from_secs(30));
    let mut tasks = Vec::new();
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let server = Arc::clone(&server);//.clone();
                let mut resources = server.resources.lock().unwrap();
                if let Some(db) = resources.get_mut::<Database>() {
                    db.save();
                    println!("save");
                }
            },
            Ok((mut stream, addr)) = listener.accept() => {
                let request = Request::new_not_tls(&mut stream).await;
                if request.is_none() {
                    continue;
                }
                let request = request.unwrap();
                let close_connection = request.headers.get("Connection") == Some(&"close".to_string());
                if close_connection {
                    println!("close connection");
                    break;
                }

                let value = Arc::clone(&server);//.clone();
                let client_task = tokio::spawn(async move {
                    let response = handle_connection(
                        request,
                        value,
                    ).await;

                    if response.is_err() {
                        return;
                    }

                    let response = response.unwrap();

                    let content = response.get();
                    let data = content.as_bytes();
                    stream.write_all(data).await.unwrap();

                    if !response.body.is_empty() {
                        let data = response.body.as_slice();
                        stream.write_all(data).await.unwrap();
                    }
                    stream.flush().await.unwrap();
                });
                tasks.push(client_task);
            },
            _ = cancel_token.cancelled() => {
                println!("cancelled");
                break;
            }
        }
    }
}
