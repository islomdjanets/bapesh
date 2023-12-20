use std::{collections::HashSet, str::FromStr, error::Error, fmt::Display};
use crate::{handshake::{Method, Status_Code, Response, Request, header_value_try_into_method, Response_Error}, server::{Resources}};

static ACCESS_CONTROL_ALLOW_HEADERS: &str = "Access-Control-Allow-Headers";
static ACCESS_CONTROL_REQUEST_HEADERS: &str = "Access-Control-Request-Headers";
static ACCESS_CONTROL_ALLOW_CREDENTIALS: &str = "Access-Control-Allow-Credentials";
static ACCESS_CONTROL_MAX_AGE: &str = "Access-Control-Max-Age";
static ACCESS_CONTROL_REQUEST_METHOD: &str = "Access-Control-Request-Method";
static ACCESS_CONTROL_ALLOW_METHODS: &str = "Access-Control-Allow-Methods"; 
static ACCESS_CONTROL_ALLOW_ORIGIN: &str = "Access-Control-Allow-Origin";

/// An enum signifying that some of type `T` is allowed, or `All` (anything is allowed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum All_Or_Some<T> {
    All,
    Some(T),
}

/// Default as `AllOrSome::All`.
impl<T> Default for All_Or_Some<T> {
    fn default() -> Self {
        All_Or_Some::All
    }
}

impl<T> All_Or_Some<T> {
    pub fn is_all(&self) -> bool {
        matches!(self, All_Or_Some::All)
    }

    pub fn is_some(&self) -> bool {
        !self.is_all()
    }

    pub fn as_ref(&self) -> Option<&T> {
        match *self {
            All_Or_Some::All => None,
            All_Or_Some::Some(ref t) => Some(t),
        }
    }

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

impl Display for Cors_Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl Error for Cors_Error {
    
}

// impl Response_Error for Cors_Error {
//     fn status_code(&self) -> Status_Code {
//         Status_Code::BadRequest
//     }
//
//     fn error(&self) -> Response {
//         Response::with_body(self.status_code(), self.to_string()).map_into_boxed_body()
//     }
// }

#[derive(Debug, Clone)]
pub struct Cors {
    origins: All_Or_Some<String>,
    methods: HashSet<Method>,
    headers: All_Or_Some<HashSet<String>>,

    methods_baked: Option<String>,
    headers_baked: Option<String>,

    allow_credentials: bool,
    max_age: Option<usize>,
    send_wildcard: bool,
    fairing_route_base: String,
    fairing_rout_rank: usize,
    vary_header: bool,
    block_on_origin_mismatch: bool,
}

impl Cors {
    pub fn new() -> Self {
        Self {
            origins: All_Or_Some::All,
            methods: HashSet::new(),
            headers: All_Or_Some::All,

            methods_baked: None,
            headers_baked: None,

            allow_credentials: false,
            max_age: None,//Some(3600),
            send_wildcard: true,
            fairing_route_base: "".into(),
            fairing_rout_rank: 128,
            vary_header: false,
            block_on_origin_mismatch: false,
        }
    }

    fn bake( &mut self ) {
        if self.allow_credentials && self.send_wildcard && self.origins.is_all() {
            println!(
                "Illegal combination of CORS options: credentials can not be supported when all \
                origins are allowed and `send_wildcard` is enabled."
                );
            // return future::err(());
        }

        // bake allowed headers value if Some and not empty
        match self.headers.as_ref() {
            Some(header_set) if !header_set.is_empty() => {
                let allowed_headers_str = intersperse_header_values(header_set);
                // Rc::make_mut(&mut self).allowed_headers_baked = Some(allowed_headers_str);
                self.headers_baked = Some(allowed_headers_str);
            }
            _ => {}
        }

        // bake allowed methods value if not empty
        // if !self.methods.is_empty() {
        //     let allowed_methods_str = intersperse_header_values(&self.methods);
        //     // Rc::make_mut(&mut inner).allowed_methods_baked = Some(allowed_methods_str);
        //     self.methods_baked = Some(allowed_methods_str);
        // }

        // bake exposed headers value if Some and not empty
        // match self.expose_headers.as_ref() {
        //     Some(header_set) if !header_set.is_empty() => {
        //         let expose_headers_str = intersperse_header_values(header_set);
        //         Rc::make_mut(&mut inner).expose_headers_baked = Some(expose_headers_str);
        //     }
        //     _ => {}
        // }
    }

    pub fn origins(mut self, origins: All_Or_Some<String> ) -> Self {
        self.origins = origins;
        self
    }

    pub fn methods(mut self, methods: Vec<Method> ) -> Self {
        for method in methods {
            self.methods.insert(method );
        }
        self
    }

    fn validate_origin( &self, request: &Request ) -> Result<bool, Cors_Error> {
        // return early if all origins are allowed or get ref to allowed origins set
        // #[allow(clippy::mutable_key_type)]
        let allowed_origins = match &self.origins {
            // All_Or_Some::All if self.allowed_origins_fns.is_empty() => return Ok(true),
            All_Or_Some::All => return Ok(true),
            All_Or_Some::Some(allowed_origins) => allowed_origins,
            // only function origin validators are defined
            // _ => &EMPTY_ORIGIN_SET,
        };

        // get origin header and try to parse as string
        match request.headers.get("Origin") {
            // origin header exists and is a string
            Some(origin) => {
                // if allowed_origins.contains(origin) || self.validate_origin_fns(origin, request) {
                if allowed_origins.contains(origin) {
                    Ok(true)
                } else if self.block_on_origin_mismatch {
                    Err(Cors_Error::Origin_Not_Allowed)
                } else {
                    Ok(false)
                }
            }

            // origin header is missing
            // note: with our implementation, the origin header is required for OPTIONS request or
            // else this would be unreachable
            None => Err(Cors_Error::Missing_Origin),
        }
    }

    fn access_control_allow_origin( &self, request: &Request ) -> Option<String> {
        let origin = request.headers.get("Origin");

        match self.origins {
            All_Or_Some::All => {
                if self.send_wildcard {
                    Some("*".to_string())
                } else {
                    // see note below about why `.cloned()` is correct
                    origin.cloned()
                }
            }

            All_Or_Some::Some(_) => {
                // since origin (if it exists) is known to be allowed if this method is called
                // then cloning the option is all that is required to be used as an echoed back
                // header value (or omitted if None)
                origin.cloned()
            }
        }
    }

    fn validate_allowed_method( &self, request: &Request) -> Result<(), Cors_Error> {
        let request_method = request
            .headers
            .get(ACCESS_CONTROL_REQUEST_METHOD)
            .map(header_value_try_into_method);

        match request_method {
            // method valid and allowed
            Some(Some(method)) if self.methods.contains(&method) => Ok(()),

            // method valid but not allowed
            Some(Some(_)) => Err(Cors_Error::Method_Not_Allowed),

            // method invalid
            Some(_) => Err(Cors_Error::Bad_Request_Method),

            // method missing so this is not a preflight request
            None => Err(Cors_Error::Missing_Request_Method),
        }
    }

    fn validate_allowed_headers(&self, request: &Request ) -> Result<(), Cors_Error> {
        // return early if all headers are allowed or get ref to allowed origins set
        // #[allow(clippy::mutable_key_type)]
        let allowed_headers = match &self.headers {
            All_Or_Some::All => return Ok(()),
            All_Or_Some::Some(allowed_headers) => allowed_headers,
        };

        // extract access control header as string
        // header format should be comma separated header names
        let request_headers = request
            .headers
            .get(ACCESS_CONTROL_REQUEST_HEADERS);
            // .map(|hdr| hdr.as_str());

        match request_headers {
            // header list is valid string
            Some(headers) => {
                // the set is ephemeral we take care not to mutate the
                // inserted keys so this lint exception is acceptable
                // #[allow(clippy::mutable_key_type)]
                let mut request_headers = HashSet::with_capacity(8);

                // try to convert each header name in the comma-separated list
                for header in headers.split(',') {
                    match header.trim().try_into() {
                        Ok(hdr) => request_headers.insert(hdr),
                        Err(_) => return Err(Cors_Error::Bad_Request_Headers),
                    };
                }

                // header list must contain 1 or more header name
                if request_headers.is_empty() {
                    return Err(Cors_Error::Bad_Request_Headers);
                }

                // request header list must be a subset of allowed headers
                if !request_headers.is_subset(allowed_headers) {
                    return Err(Cors_Error::Headers_Not_Allowed);
                }

                Ok(())
            }

            // header list is not a string
            // Err(_) => Err(Cors_Error::Bad_Request_Headers),

            // header list missing
            None => Ok(()),
        }
    }

// fn add_vary_header(headers: &mut HeaderMap) {
//     let value = match headers.get("Vary") {
//         Some(hdr) => {
//             let mut val: Vec<u8> = Vec::with_capacity(hdr.len() + 71);
//             val.extend(hdr.as_bytes());
//             val.extend(b", Origin, Access-Control-Request-Method, Access-Control-Request-Headers");
//
//             #[cfg(feature = "draft-private-network-access")]
//             val.extend(b", Access-Control-Request-Private-Network");
//
//             val.try_into().unwrap()
//         }
//
//         #[cfg(feature = "draft-private-network-access")]
//         None => HeaderValue::from_static(
//             "Origin, Access-Control-Request-Method, Access-Control-Request-Headers, \
//             Access-Control-Request-Private-Network",
//             ),
//
//             #[cfg(not(feature = "draft-private-network-access"))]
//         None => HeaderValue::from_static(
//             "Origin, Access-Control-Request-Method, Access-Control-Request-Headers",
//             ),
//     };
//
//     headers.insert(header::VARY, value);
// }
}

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_preflight( request: &Request ) -> bool {
    request
        .headers
        .get(ACCESS_CONTROL_REQUEST_METHOD)
        .and_then(header_value_try_into_method)
        .is_some()
}

pub fn middleware( response: &mut Response, resources: &mut Resources ) {
    response.headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
    // println!("add Cors headers");
}

pub fn handle_preflight( request: &Request, resources: &mut Resources ) -> Response {
    match resources.get_mut::<Cors>() {
        Some(cors_options) => {
            // let cors_options = Rc::clone(&self.inner);

            // cors_options.bake();
            let mut response = Response::new();
            match cors_options.validate_origin(request) {
                Ok(true) => {}
                Ok(false) => return response,//.error(Cors_Error::Origin_Not_Allowed),
                Err(err) => return response,//.error(err),
            };

            if let Err(err) = cors_options
                .validate_allowed_method(request)
                    .and_then(|_| cors_options.validate_allowed_headers(request))
                    {
                        return response;//.error(err);
                    }
            
            if let Some(origin) = cors_options.access_control_allow_origin(request) {
                response.headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN.to_string(), origin);
            }

            if let Some(ref allowed_methods) = cors_options.methods_baked {
                response.headers.insert(
                    ACCESS_CONTROL_ALLOW_METHODS.to_string(),
                    allowed_methods.clone(),
                );
            } else {
                let mut allowed_methods = "".to_string();
                for method in cors_options.methods.iter() {
                    allowed_methods += &method.to_string();
                    allowed_methods += ",";
                }
                allowed_methods.pop();
                response.headers.insert(
                    ACCESS_CONTROL_ALLOW_METHODS.to_string(),
                    allowed_methods.clone(),
                );
                cors_options.methods_baked = Some(allowed_methods);
            }

            if let Some(ref headers) = cors_options.headers_baked {
                response.headers.insert(ACCESS_CONTROL_ALLOW_HEADERS.to_string(), headers.clone());
            } else if let Some(headers) = request.headers.get(ACCESS_CONTROL_REQUEST_HEADERS) {
                // all headers allowed, return
                response.headers.insert(ACCESS_CONTROL_ALLOW_HEADERS.to_string(), headers.clone());
            }

            #[cfg(feature = "draft-private-network-access")]
            if inner.allow_private_network_access
                && req
                    .headers()
                    .contains_key("access-control-request-private-network")
                    {
                        res.insert_header((
                                header::HeaderName::from_static("access-control-allow-private-network"),
                                HeaderValue::from_static("true"),
                                ));
                    }

            if cors_options.allow_credentials {
                response.headers.insert(
                    ACCESS_CONTROL_ALLOW_CREDENTIALS.to_string(),
                    "true".to_string(),
                );
            }

            if let Some(max_age) = cors_options.max_age {
                response.headers.insert(ACCESS_CONTROL_MAX_AGE.to_string(), max_age.to_string());
            }

            // if cors_options.vary_header {
            //     add_vary_header(res.headers_mut());
            // }

            // request.into_response(res)
          
            response.set_status(Status_Code::NoContent);

            response

        },
        None => Response::new(),
    }
}

    // fn augment_response<B>( inner: &Inner, origin_allowed: bool, mut res: ServiceResponse<B> ) -> ServiceResponse<B> {
    //     if origin_allowed {
    //         if let Some(origin) = inner.access_control_allow_origin(res.request().head()) {
    //             res.headers_mut()
    //                 .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    //         };
    //     }
    //
    //     if let Some(ref expose) = inner.expose_headers_baked {
    //         log::trace!("exposing selected headers: {:?}", expose);
    //
    //         res.headers_mut()
    //             .insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, expose.clone());
    //     } else if matches!(inner.expose_headers, AllOrSome::All) {
    //         // intersperse_header_values requires that argument is non-empty
    //         if !res.headers().is_empty() {
    //             // extract header names from request
    //             let expose_all_request_headers = res
    //                 .headers()
    //                 .keys()
    //                 .map(|name| name.as_str())
    //                 .collect::<HashSet<_>>();
    //
    //             // create comma separated string of header names
    //             let expose_headers_value = intersperse_header_values(&expose_all_request_headers);
    //
    //             log::trace!(
    //                 "exposing all headers from request: {:?}",
    //                 expose_headers_value
    //             );
    //
    //             // add header names to expose response header
    //             res.headers_mut()
    //                 .insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, expose_headers_value);
    //         }
    //     }
    //
    //     if inner.supports_credentials {
    //         res.headers_mut().insert(
    //             header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
    //             HeaderValue::from_static("true"),
    //         );
    //     }
    //
    //     #[cfg(feature = "draft-private-network-access")]
    //     if inner.allow_private_network_access
    //         && res
    //             .request()
    //             .headers()
    //             .contains_key("access-control-request-private-network")
    //     {
    //         res.headers_mut().insert(
    //             header::HeaderName::from_static("access-control-allow-private-network"),
    //             HeaderValue::from_static("true"),
    //         );
    //     }
    //
    //     if inner.vary_header {
    //         add_vary_header(res.headers_mut());
    //     }
    //     res
    // }

/// Only call when values are guaranteed to be valid header values and set is not empty.
pub(crate) fn intersperse_header_values<T>(val_set: &HashSet<T>) -> String
where
    T: AsRef<str>,
{
    debug_assert!(
        !val_set.is_empty(),
        "only call `intersperse_header_values` when set is not empty"
    );

    val_set
        .iter()
        .fold(String::with_capacity(64), |mut acc, val| {
            acc.push_str(", ");
            acc.push_str(val.as_ref());
            acc
        })
        // set is not empty so string will always have leading ", " to trim
        [2..]
        .try_into()
        // all method names are valid header values
        .unwrap()
}

