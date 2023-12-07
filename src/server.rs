use std::{ io::*, str, net::{TcpStream, TcpListener}, collections::HashMap};

use crate::handshake::{Method, Request, Response};

pub type Resources = HashMap<String, Box<dyn Resource>>;
pub type Handler = fn(&Request, &mut Resources) -> Result<Response>;

pub struct Route {
    method: Method,
    uri: fn(&Request) -> bool,
    handler: Handler
}
//
// pub struct Server {
//     pub address: SocketAddr,
//     pub routes: HashMap<Route, Route_Handler>,
// }
//
// impl Server {
//    pub async fn run(&self) -> std::io::Result<()> {
//        let listener: TcpListener = TcpListener::bind(self.address).await?;
//        println!("{} listening on port:{}", "server".green(), self.address.to_string().red());
//
//        loop {
//             let ( mut stream: TcpStream, _ ) = listener.accept().await?;
//             let routes = self.routes.clone(); // WTF???!!!
//             let middleware = Arc::clone(&self.middleware);
//
//             tokio::spawn( async move {
//                 let mut buffer = [0; 1024];
//                 let _ = stream.read(&mut buffer).await.unwrap();
//
//                 let request = parse_request(&buffer).unwrap();
//
//                 let future_response = handle_route( request,)
//             })
//        }
//    }
// }
//

pub trait Resource {
   // fn get_session( &mut self, project_name: String ) -> Option<&mut Session>;
   // fn add_session( &mut self, key: String, value: Session );
   
    fn get_resource( &mut self, project_name: String ) -> Option<&mut dyn Resource>;
    fn add_resource( &mut self, key: String, value: dyn Resource );
}

pub struct Server {
    routes: Vec<Route>,
    resources: HashMap<String, Box<dyn Resource>>
}

impl Server {
    pub fn new() -> Self {
        Server {
            routes: vec![],
            resources: HashMap::new()
        }
    }

    pub fn add_resourse( &mut self, name: String, resource: Box<dyn Resource> ) -> &mut Self {
        self.resources.insert(name, resource);
        self
    }

    pub async fn bind(&mut self, host: &str, port: u16, multithreaded: bool) -> &mut Self {
        //println!("bind");

        // let host_array = host.split('.');
        //let address = SocketAddr::from((host, port));

        let listener = TcpListener::bind(format!("{host}:{port}")).unwrap();

        if multithreaded {
            // let pool = ThreadPool::new(4);

            // for stream in listener.incoming() {
            //     let stream = stream.unwrap();

            //     pool.execute(|| {
            //         self.handle_connection(stream);
            //     });
            // }
        } else {
            for stream in listener.incoming() {
                let stream = stream.unwrap();

                self.handle_connection(stream).await;
            }
        }

        // loop {
        //     let (socket, _) = listener.accept();
        //     self.handle_connection(socket).await;
        // }

        self
    }

    async fn handle_connection(&mut self, mut stream: TcpStream) {
        let request = Request::new(&stream);

        // stream // socket

        for route in self.routes.iter() {
            if route.method == request.method && (route.uri)(&request) {
                if let Ok( response ) = (route.handler)(&request, &mut self.resources) {
                    println!("OK");
                    stream.write_all(response.get().as_bytes()).unwrap();
                    if !response.body.is_empty() {
                        stream.write_all(&response.body).unwrap();
                        println!("write body");
                    }
                    stream.flush().unwrap();
                };
                return;
            }
        }
    }

    pub fn run(&self) -> &Self {
        //println!("run");
        self
    }

    pub fn get(&mut self, uri: fn( request: &Request ) -> bool, handler: Handler ) -> &mut Self {
        self.routes.push(Route {
            method: Method::GET,
            uri,
            handler,
        });

        self
    }

    pub fn post(&mut self, uri: fn( request: &Request ) -> bool, handler: Handler ) -> &mut Self {
        self.routes.push(Route {
            method: Method::POST,
            uri,
            handler,
        });

        self
    }

    pub fn delete(&mut self, uri: fn( request: &Request ) -> bool, handler: Handler ) -> &mut Self {
        self.routes.push(Route {
            method: Method::DELETE,
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
