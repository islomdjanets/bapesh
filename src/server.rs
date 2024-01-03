use core::fmt;
use std::{ io::*, ops::Deref, str, net::{TcpStream, TcpListener, Shutdown}, collections::HashMap, thread, sync::{Mutex, Arc}, any::{TypeId, Any}, hash::{BuildHasherDefault, Hasher}};

use crate::{handshake::{Method, Request, Response}, cors, responder::Responder, ws};

pub type Handler = fn(&Request, &mut Resources) -> Response;
pub type Check = fn( &Request ) -> bool;
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
pub type Services = Arc<Mutex<Vec<Service>>>;

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

#[derive(PartialEq, Eq, Hash)]
pub struct Route {
    method: Method,
    uri: Check,
    handler: Handler
}

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

pub struct Service {
    uri: Check,
    handle: fn( &mut TcpStream, Arc<Mutex<Resources>> ),
}

pub struct Server {
    host: String,
    port: u16,
    routes: Routes,
    middlewares: Middlewares,
    resources: Arc<Mutex<Resources>>,
    services: Services,
}

impl Server {
    pub fn new() -> Self {
        Server {
            host: "127.0.0.1".into(),
            port: 7878,
            routes: Arc::new(Mutex::new(Vec::new())),
            middlewares: Arc::new(Mutex::new(Vec::new())),
            resources: Arc::new(Mutex::new(Resources::new())),
            services: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn cors( &mut self, options: cors::Cors ) -> &mut Self {
        
        self.add_resourse(options);
        self.add_middleware(cors::middleware);
        self.add_route( Route {
            method: Method::OPTIONS,
            uri: cors::is_preflight,
            handler: cors::handle_preflight
        });
        self
    }

    pub fn ws( &mut self, options: ws::WebSocket ) -> &mut Self {

        self.add_route( Route {
            method: Method::GET,
            uri: options.uri,
            handler: ws::update_to_websocket
        });

        self.service( Service {
            uri: options.uri,
            handle: ws::handle_connection,
        });

        self.add_resourse(options);
        self
    }

    pub fn service( &mut self, service: Service ) -> &mut Self {
        self.services.lock().unwrap().push(service);
        self
    }

    pub fn add_middleware( &mut self, middleware: Middleware ) -> &mut Self {
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

    pub fn host(&mut self, host: &str ) -> &mut Self {
        self.host = host.into();
        self
    }

    pub fn port(&mut self, port: u16 ) -> &mut Self {
        self.port = port;
        self
    }

    pub async fn run_multi( self ) {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).unwrap();
        for stream in listener.incoming() {

            // thread::spawn( move || {
            //     handle_connection(
            //         stream.unwrap(),
            //         Arc::clone( &self.routes ),
            //         Arc::clone( &self.resources ),
            //         Arc::clone( &self.middlewares ),
            //     );
            // });
        }
    }

    pub fn run( &mut self ) {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).unwrap();

        for stream in listener.incoming() {
            handle_connection(
                stream.unwrap(),
                Arc::clone( &self.routes ),
                Arc::clone( &self.resources ),
                Arc::clone( &self.middlewares ),
                Arc::clone( &self.services ),
            );
        }
    }

    fn add_route(&mut self, route: Route ) { 
        let mut routes = self.routes.lock().unwrap();

        // match routes.get_mut(&method) {
        //     Some(routes) => routes.push(route),
        //     None => {
        //         routes.insert(method.clone(), Vec::new());
        //         routes.get_mut(&method).unwrap().push(route);
        //     },
        // };
        routes.push( route );
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

fn handle_connection( mut stream: TcpStream, routes: Routes, resources: Arc<Mutex<Resources>>, middlewares: Middlewares, services: Services ) {
    let request = Request::new(&stream);

    // if let Some(routes) = routes.lock().unwrap().get(&request.method) {
    let routes = routes.lock().unwrap();
    for route in routes.iter() {
        if route.method == request.method && (route.uri)(&request) {

            let mut resources = resources.lock().unwrap(); 
            let mut response = (route.handler)(&request, &mut resources);

            let middlewares = middlewares.lock().unwrap();
            for middleware in middlewares.iter() {
                (middleware)(&mut response, &mut resources);
            }

            stream.write_all(response.get().as_bytes()).unwrap();
            if !response.body.is_empty() {
                stream.write_all(&response.body).unwrap();
            }
            stream.flush().unwrap();
            // stream.shutdown(Shutdown::Both).unwrap();
            // return; 
        } 
    }

    let services = services.lock().unwrap();
    for service in services.iter() {
        if (service.uri)(&request) {
            (service.handle)( &mut stream, resources );
            return;
        }
    }
}
