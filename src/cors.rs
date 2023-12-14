use std::{collections::HashSet, io::Result};
use crate::{handshake::{Method, Status_Code, Response, Request}, server::{Resources}};

// use derive_more::{Display, Error};

/// An enum signifying that some of type `T` is allowed, or `All` (anything is allowed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum All_Or_Some<T> {
    /// Everything is allowed. Usually equivalent to the `*` value.
    All,

    /// Only some of `T` is allowed
    Some(T),
}

/// Default as `AllOrSome::All`.
impl<T> Default for All_Or_Some<T> {
    fn default() -> Self {
        All_Or_Some::All
    }
}

impl<T> All_Or_Some<T> {
    /// Returns whether this is an `All` variant.
    pub fn is_all(&self) -> bool {
        matches!(self, All_Or_Some::All)
    }

    /// Returns whether this is a `Some` variant.
    #[allow(dead_code)]
    pub fn is_some(&self) -> bool {
        !self.is_all()
    }

    /// Provides a shared reference to `T` if variant is `Some`.
    pub fn as_ref(&self) -> Option<&T> {
        match *self {
            All_Or_Some::All => None,
            All_Or_Some::Some(ref t) => Some(t),
        }
    }

    /// Provides a mutable reference to `T` if variant is `Some`.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        match *self {
            All_Or_Some::All => None,
            All_Or_Some::Some(ref mut t) => Some(t),
        }
    }
}

/// Errors that can occur when processing CORS guarded requests.
// #[derive(Debug, Clone, Display, Error)]
#[derive(Debug, Clone)]
// // #[non_exhaustive]
pub enum Cors_Error {
    /// Allowed origin argument must not be wildcard (`*`).
    // #[display(fmt = "`allowed_origin` argument must not be wildcard (`*`)")]
    Wildcard_Origin,

    /// Request header `Origin` is required but was not provided.
    // #[display(fmt = "Request header `Origin` is required but was not provided")]
    Missing_Origin,

    /// Request header `Access-Control-Request-Method` is required but is missing.
    // #[display(fmt = "Request header `Access-Control-Request-Method` is required but is missing")]
    Missing_Request_Method,

    /// Request header `Access-Control-Request-Method` has an invalid value.
    // #[display(fmt = "Request header `Access-Control-Request-Method` has an invalid value")]
    Bad_Request_Method,

    /// Request header `Access-Control-Request-Headers` has an invalid value.
    // #[display(fmt = "Request header `Access-Control-Request-Headers` has an invalid value")]
    Bad_Request_Headers,

    /// Origin is not allowed to make this request.
    // #[display(fmt = "Origin is not allowed to make this request")]
    Origin_Not_Allowed,

    /// Request method is not allowed.
    // #[display(fmt = "Requested method is not allowed")]
    Method_Not_Allowed,

    /// One or more request headers are not allowed.
    // #[display(fmt = "One or more request headers are not allowed")]
    Headers_Not_Allowed,
}

// impl ResponseError for CorsError {
//     fn status_code(&self) -> Status_Code {
//         Status_Code::BAD_REQUEST
//     }
//
//     fn error_response(&self) -> Response {
//         Response::with_body(self.status_code(), self.to_string()).map_into_boxed_body()
//     }
// }

#[derive(Debug, Clone)]
pub struct Cors {
    origins: Vec<String>,
    methods: HashSet<Method>,
    headers: Vec<String>,
    allow_credentials: bool,
    max_age: Option<usize>,
    send_wildcard: String,
    fairing_route_base: String,
    fairing_rout_rank: usize,
}

// impl Resource for Cors {
//     fn get_resource( &mut self, project_name: String ) -> Option<&mut dyn Resource> {
//         Some(self)    
//     } 
//
//     fn add_resource( &mut self, key: String, _: Box<dyn Resource> ) {
//         
//     }
// }

impl Cors {
    pub fn new() -> Self {
        Self {
            origins: vec![],
            methods: HashSet::new(),
            headers: vec![],
            allow_credentials: false,
            max_age: None,
            send_wildcard: "".into(),
            fairing_route_base: "".into(),
            fairing_rout_rank: 128,
        }
    }

    pub fn origins(mut self, origins: Vec<String> ) -> Self {
        self.origins.append(&mut origins.clone());
        self
    }

    pub fn methods(mut self, methods: Vec<Method> ) -> Self {
        for method in methods {
            self.methods.insert(method );
        }
        self
    }

    // pub fn get( self ) -> Self {
    //     self
    // }
}

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_cors( request: &Request ) -> bool {
    println!("cors");
    // match self.cor {
    //     
    // } 
    true
}

// fn verify_cors( request: &Request, resources: &mut Resources ) -> impl Responder {
pub fn verify_cors( request: &Request, resources: &mut Resources ) -> Response {
    match resources.get::<Cors>() {
        Some(cors_options) => {
            // let cors = cors_options;
            println!("verify cors");
            println!("{:?}", cors_options );
            Response::new()
        },
        None => Response::new(),
    }             
}

