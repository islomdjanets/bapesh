use std::{str::{self, FromStr}, net::TcpStream, io::{Read, BufReader}, fmt::{Display, Error}, collections::HashMap};
use crate::{driver::Driver, json::JSON};

const HTTP_VERSION: &str = "HTTP/1.1";

#[derive(Debug, PartialEq)]
pub enum Status_Code {
    //#[default]
    Continue = 100,
    SwitchingProtocols = 101,

    OK = 200,
    Created = 201,
    Accepted = 202,
    Non_AuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,

    MultipleChoices = 300,
    MovedPermanently = 301,
    MovedTemporarily = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,

    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    RequestEntityTooLarge  = 413,
    Request_URITooLarge  = 414,
    UnsupportedMediaType = 415,

    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTime_out = 504,
    HTTPVersionnotsupported = 505,
}

impl Status_Code {
    fn get_code(&self) -> u16 {
        match self {
            Status_Code::Continue => 100,
            Status_Code::SwitchingProtocols => 101,
            Status_Code::OK => 200,
            Status_Code::Created => todo!(),
            Status_Code::Accepted => todo!(),
            Status_Code::Non_AuthoritativeInformation => todo!(),
            Status_Code::NoContent => 204,
            Status_Code::ResetContent => todo!(),
            Status_Code::PartialContent => todo!(),
            Status_Code::MultipleChoices => todo!(),
            Status_Code::MovedPermanently => todo!(),
            Status_Code::MovedTemporarily => todo!(),
            Status_Code::SeeOther => todo!(),
            Status_Code::NotModified => todo!(),
            Status_Code::UseProxy => todo!(),
            Status_Code::BadRequest => 400,
            Status_Code::Unauthorized => todo!(),
            Status_Code::Forbidden => todo!(),
            Status_Code::NotFound => todo!(),
            Status_Code::MethodNotAllowed => todo!(),
            Status_Code::NotAcceptable => todo!(),
            Status_Code::Conflict => todo!(),
            Status_Code::Gone => todo!(),
            Status_Code::LengthRequired => todo!(),
            Status_Code::PreconditionFailed => todo!(),
            Status_Code::RequestEntityTooLarge => todo!(),
            Status_Code::Request_URITooLarge => todo!(),
            Status_Code::UnsupportedMediaType => todo!(),
            Status_Code::InternalServerError => todo!(),
            Status_Code::NotImplemented => todo!(),
            Status_Code::BadGateway => todo!(),
            Status_Code::ServiceUnavailable => todo!(),
            Status_Code::GatewayTime_out => todo!(),
            Status_Code::HTTPVersionnotsupported => todo!(),
        }
    } 
}

impl ToString for Status_Code {
    fn to_string(&self) -> String {
        let status = match self {
            Status_Code::Continue => "Continue",
            Status_Code::SwitchingProtocols => "Switching Protocols",
            Status_Code::OK => "OK",
            Status_Code::Created => "Created",
            Status_Code::Accepted => "Accepted",
            Status_Code::Non_AuthoritativeInformation => "Non-Authoritative Information",
            Status_Code::NoContent => "No Content",
            Status_Code::ResetContent => "Reset Content",
            Status_Code::PartialContent => "Partial Content",
            Status_Code::MultipleChoices => "Multiple Choices",
            Status_Code::MovedPermanently => "Moved Permanently",
            Status_Code::MovedTemporarily => "Moved Temporarily",
            Status_Code::SeeOther => "See Other",
            Status_Code::NotModified => "Not Modified",
            Status_Code::UseProxy => "Use Proxy",
            Status_Code::BadRequest => "Bad Request",
            Status_Code::Unauthorized => "Unauthorized",
            Status_Code::Forbidden => "Forbidden",
            Status_Code::NotFound => "Not Found",
            Status_Code::MethodNotAllowed => "Method Not Allowed",
            Status_Code::NotAcceptable => "Not Acceptable",
            Status_Code::Conflict => "Conflict",
            Status_Code::Gone => "Gone",
            Status_Code::LengthRequired => "Length Required",
            Status_Code::PreconditionFailed => "Precondition Failed",
            Status_Code::RequestEntityTooLarge => "Request Entity Too Large",
            Status_Code::Request_URITooLarge => "Request-URI Too Large",
            Status_Code::UnsupportedMediaType => "Unsupported Media Type",
            Status_Code::InternalServerError => "Internal Server Error",
            Status_Code::NotImplemented => "Not Implemented",
            Status_Code::BadGateway => "Bad Gateway",
            Status_Code::ServiceUnavailable => "Service Unavailable",
            Status_Code::GatewayTime_out => "Gateway Time-out",
            Status_Code::HTTPVersionnotsupported => "HTTP Version not supported",
            //_ => "undefined".into(),
        };
        status.into()
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    PATCH,
    OTHER(String),
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "CONNECT" => Ok(Method::CONNECT),
            "TRACE" => Ok(Method::TRACE),
            "PATCH" => Ok(Method::PATCH),
            _ => Ok(Method::OTHER(s.into())),
            //_ => ()
        }
    }
}

impl ToString for Method {
    fn to_string(&self) -> String {
        let value = match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
            Method::CONNECT => "CONNECT",
            Method::TRACE => "TRACE",
            Method::PATCH => "PATCH",
            _ => "OTHER",
        };

        value.to_string()
    } 
}

pub fn header_value_try_into_method( hdr: &String ) -> Option<Method> {
    match Method::from_str(hdr) {
        Ok(method) => Some(method),
        Err(_) => None,
    }
}

pub struct Request {
    pub method: Method,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>
}

impl Request {
    pub fn new(stream: &TcpStream) -> Self {
        // let mut buffer = [0; 8192];
        let mut buffer = [0; 2048];
        // let mut buffer = Vec::new();

        // stream.read_exact(&mut buffer).unwrap();
        // stream.read(&mut buffer).unwrap();
        BufReader::new(stream).read(&mut buffer).unwrap();

        // let size = stream.read_to_end(&mut buffer).unwrap();
        // println!("{size}");
        // let mut buffer = [0; 1024];
        // stream.read_to_end(&mut buffer).unwrap();
         
        let request = String::from_utf8_lossy(&buffer);

        let mut parts = request.split("\r\n");

        let mut status = parts.nth(0).unwrap().split_whitespace();
        let mut headers = HashMap::new();
        let mut body = Vec::new();

        for header in parts {
            let mut key_value = header.split(':');
            if key_value.clone().count() != 2 {
                body = header.into();
                continue;
            }
            headers.insert(
                String::from(key_value.nth(0).unwrap().trim()),
                String::from(key_value.nth(0).unwrap().trim())
            );
        }

        Request {
            method: Method::from_str(status.nth(0).unwrap()).unwrap(),
            uri: status.nth(0).unwrap_or("").into(),
            headers,
            body
        }
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "STATUS -> Method: {:?} | path: {} | version: {} HEADERS -> {:#?} BODY -> {}",
            &self.method, &self.uri, &HTTP_VERSION, self.headers, &str::from_utf8(&self.body).unwrap()
        )
    }
}

pub trait Response_Error {
    fn status_code(&self) -> Status_Code;

    fn error(&self) -> Response;
}

#[derive(Debug)]
pub struct Response {
    pub status: Status_Code,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>
}

impl Response {
    pub fn new() -> Self {
        Self { 
            status: Status_Code::OK, 
            headers: HashMap::new(),
            body: Vec::new()
        }
    }
    
    pub fn text( content: String ) -> Response {
        let content_type = Response::get_mime("text").into();
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".into(), content_type);
        headers.insert("Content-Length".into(), content.len().to_string());
        Self { 
            status: Status_Code::OK, 
            headers,
            body: content.into_bytes()
        }
    }

    pub fn json( content: JSON ) -> Response {
        let content_type = Response::get_mime("json").into();
        let bytes = content.to_string().as_bytes().to_vec();

        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".into(), content_type);
        headers.insert("Content-Length".into(), bytes.len().to_string());
        Self { 
            status: Status_Code::OK, 
            headers,
            body: bytes
        }
    }

    pub fn html( content: Vec<u8> ) -> Response {
        let content_type = Response::get_mime("html").into();
        let mut headers: HashMap<String, String> = HashMap::new();
        headers.insert("Content-Type".into(), content_type);
        headers.insert("Content-Length".into(), content.len().to_string());
        Self { 
            status: Status_Code::OK, 
            headers,
            body: content
        }
    }

    pub fn get_mime( format: &str ) -> &str {
        match format {
            "text" => "text/plain",
            "html" => "text/html",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "webp" => "image/webp",
            "jpg" => "image/jpeg",
            "json" => "text/json",
            "js" => "text/javascript",
            "wasm" => "application/wasm",
            _ => "undefined"
        }        
    } 

    pub fn is_binary( mime: &str ) -> bool {
        matches!(mime, "image/png" | "image/webp" | "image/jpeg")
    }

    pub fn file( path: String ) -> Response {
        //let format = ;
        let content_type = Response::get_mime(&path.split('.').last().unwrap().to_string()).into();
        //println!("format: {}", format);
        match Driver::read_binary(&path) {
            Ok(content) => {
                let mut headers: HashMap<String, String> = HashMap::new();
                headers.insert("Content-Type".into(), content_type);
                headers.insert("Content-Length".into(), content.len().to_string());

                //println!("{:?}", str::from_utf8(&content).unwrap());
                Self { 
                    status: Status_Code::OK, 
                    headers,
                    //content_type: "js".into(),
                    body: content
                }
            }
            Err(error) => {
                //status = "HTTP/1.1 404 Not Found";
                // println!("path: {} | {}", path, error );
                //"NotFound".into()

                let len = path.len() - 3;
                let path = path[0..len].to_string();// .nth(0).unwrap().to_string();
                //println!("directory: {}", path );

                if Driver::is_directory(&path) {
                    let new_path = format!("{}{}", path, "/main.js" );
                    //println!("new path: {}", new_path );
                    return Response::file(new_path);
                }
                
                Response::new()
            }
        }
    }

    pub fn get( &self ) -> String {
        let status = format!("{} {} {}", HTTP_VERSION, self.status.get_code(), self.status.to_string());

        let out = format!(
            "{}\r\n{}\r\n",
            status,
            self.get_headers(),
        );
        
        // println!("{out}");
        // println!("---------------------------------------");
        out
    }

    fn get_headers( &self ) -> String {
        let mut result = String::from("");

        for header in self.headers.iter() {
            result = result + &format!("{}: {}\r\n", header.0, header.1);
        }

        result
    }

    pub fn set_status( &mut self, status_code: Status_Code ) {
        self.status = status_code;
    }

    pub fn error<E: Into<Error>>(self, err: E) -> Self {
        // ServiceResponse::from_err(err, self.request)
        self
    }
}

// impl FromStr for Response {
//     type Err = ();
//
//     fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
//         Ok(Response::text(s.to_string())) 
//     } 
// }

impl From<Status_Code> for Response {
    fn from(val: Status_Code) -> Self {
        let mut response = Response::new();
        response.set_status(val);
        response
    } 
}

impl From<&str> for Response {
    fn from(val: &str) -> Self {
        Response::text(val.to_string())
    } 
}

impl From<String> for Response {
    fn from(val: String) -> Self {
        Response::text(val)
    } 
}

impl From<JSON> for Response {
    fn from(value: JSON) -> Self {
        Response::json( value )
    } 
}

impl Response_Error for Request {
    fn status_code(&self) -> Status_Code {
        todo!()
    }

    fn error(&self) -> Response {
        todo!()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "not implemented!",
        )
    }
}
