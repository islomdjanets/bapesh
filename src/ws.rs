use std::{collections::HashMap, fmt, sync::{Arc, Mutex}, io::{Result}};
use crate::{handshake::{Request, Response, Status_Code}, server::{Check, Resources, Service}};

use sha1::Digest;
use tokio::{io::AsyncReadExt, net::TcpStream};

static WS_MAGIC_STRING : &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
static BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

const SEVEN_BITS_INTEGER_MARKER: u8 = 125;
const SIXTEEN_BITS_INTEGER_MARKER: u8 = 126;
const SIXTYFOUR_BITS_INTEGER_MARKER: u8 = 127;

// const MAX_SIXTEEN_BITS_INTEGER: u8 = 2 ** 16;

const MASK_KEY_BYTES_LENGTH: u8 = 4;

const FIRST_BIT: u8 = 128;
const OPCODE_TEXT: u8 = 0x01; // 1 bit in binary
                          //
fn hash_key( key : &String ) -> String {
    let mut hasher = sha1::Sha1::new();

    hasher.input(key.as_bytes());
    hasher.input(WS_MAGIC_STRING.as_bytes());

    encode_base64(&hasher.result())
}

fn encode_base64(data: &[u8]) -> String {
    let len = data.len();
    let mod_len = len % 3;

    let mut encoded = vec![b'='; (len + 2) / 3 * 4];
    {
        let mut in_iter = data[..len - mod_len].iter().map(|&c| u32::from(c));
        let mut out_iter = encoded.iter_mut();

        let enc = |val| BASE64[val as usize];
        let mut write = |val| *out_iter.next().unwrap() = val;

        while let (Some(one), Some(two), Some(three)) =
            (in_iter.next(), in_iter.next(), in_iter.next())
        {
            let g24 = one << 16 | two << 8 | three;
            write(enc((g24 >> 18) & 63));
            write(enc((g24 >> 12) & 63));
            write(enc((g24 >> 6) & 63));
            write(enc(g24 & 63));
        }

        match mod_len {
            1 => {
                let pad = (u32::from(data[len - 1])) << 16;
                write(enc((pad >> 18) & 63));
                write(enc((pad >> 12) & 63));
            }
            2 => {
                let pad = (u32::from(data[len - 2])) << 16 | (u32::from(data[len - 1])) << 8;
                write(enc((pad >> 18) & 63));
                write(enc((pad >> 12) & 63));
                write(enc((pad >> 6) & 63));
            }
            _ => (),
        }
    }

    String::from_utf8(encoded).unwrap()
}

// pub fn is_websocket_update( request: &Request ) -> bool {
//     if let Some(header) = request.headers.get("Connection") {
//         header == "Upgrade"
//     } else {
//         false
//     }
// }
//
// pub fn update_to_websocket( request: &Request, _: &mut Resources ) -> Response {
//     let key = request.headers.get("Sec-WebSocket-Key").unwrap();
//     println!("websocket key : {key}");
//
//     let hashed_key = hash_key(key);
//
//     let mut headers: HashMap<String, String> = HashMap::new();
//     headers.insert("Connection".into(), "Upgrade".into());
//     headers.insert("Upgrade".into(), "websocket".into());
//     headers.insert("Sec-WebSocket-Accept".into(), hashed_key);
//     
//     Response {
//         status: Status_Code::SwitchingProtocols,
//         headers,
//         body: Vec::new(),
//     }
// }


//////// Protocol

use self::Close_Code::*;
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Close_Code {
    Normal,
    Away,
    Protocol,
    Unsupported,
    Status,
    Abnormal,
    Invalid,
    Policy,
    Size,
    Extension,
    Error,
    Restart,

    Again,
    // #[doc(hidden)]
    Tls,
    // #[doc(hidden)]
    Empty,
    // #[doc(hidden)]
    Other(u16),
}

impl From<Close_Code> for u16 {
    fn from(val: Close_Code) -> Self {
        match val {
            Normal => 1000,
            Away => 1001,
            Protocol => 1002,
            Unsupported => 1003,
            Status => 1005,
            Abnormal => 1006,
            Invalid => 1007,
            Policy => 1008,
            Size => 1009,
            Extension => 1010,
            Error => 1011,
            Restart => 1012,
            Again => 1013,
            Tls => 1015,
            Empty => 0,
            Other(code) => code,
        }
    }
}

impl From<u16> for Close_Code {
    fn from(code: u16) -> Self {
        match code {
            1000 => Normal,
            1001 => Away,
            1002 => Protocol,
            1003 => Unsupported,
            1005 => Status,
            1006 => Abnormal,
            1007 => Invalid,
            1008 => Policy,
            1009 => Size,
            1010 => Extension,
            1011 => Error,
            1012 => Restart,
            1013 => Again,
            1015 => Tls,
            0 => Empty,
            _ => Other(code),
        }
    }
}

use self::Op_Code::*;
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Op_Code {
    /// Indicates a continuation frame of a fragmented message.
    Continue,
    /// Indicates a text data frame.
    Text,
    /// Indicates a binary data frame.
    Binary,
    /// Indicates a close control frame.
    Close,
    /// Indicates a ping control frame.
    Ping,
    /// Indicates a pong control frame.
    Pong,
    /// Indicates an invalid opcode was received.
    Bad,
}

impl Op_Code {
    /// Test whether the opcode indicates a control frame.
    pub fn is_control(&self) -> bool {
        !matches!(*self, Text | Binary | Continue)
    }
}

impl fmt::Display for Op_Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Continue => write!(f, "CONTINUE"),
            Text => write!(f, "TEXT"),
            Binary => write!(f, "BINARY"),
            Close => write!(f, "CLOSE"),
            Ping => write!(f, "PING"),
            Pong => write!(f, "PONG"),
            Bad => write!(f, "BAD"),
        }
    }
}

impl From<Op_Code> for u8 {
    fn from(val: Op_Code) -> Self {
        match val {
            Continue => 0,
            Text => 1,
            Binary => 2,
            Close => 8,
            Ping => 9,
            Pong => 10,
            Bad => {
                debug_assert!(
                    false,
                    "Attempted to convert invalid opcode to u8. This is a bug."
                );
                8 // if this somehow happens, a close frame will help us tear down quickly
            }
        }
    }
}

impl From<u8> for Op_Code {
    fn from(byte: u8) -> Self {
        match byte {
            0 => Continue,
            1 => Text,
            2 => Binary,
            8 => Close,
            9 => Ping,
            10 => Pong,
            _ => Bad,
        }
    }
}

////////


pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    // Close(Option<CloseFrame<'static>>),
    Frame(Frame),
}

impl Message {
    pub fn text<S>(string: S) -> Message
    where
        S: Into<String>,
    {
        Message::Text(string.into())
    }

    pub fn binary<B>(bin: B) -> Message
    where
        B: Into<Vec<u8>>,
    {
        Message::Binary(bin.into())
    }

    pub fn is_text(&self) -> bool {
        matches!(*self, Message::Text(_))
    }

    pub fn is_binary(&self) -> bool {
        matches!(*self, Message::Binary(_))
    }

    pub fn is_ping(&self) -> bool {
        matches!(*self, Message::Ping(_))
    }
    pub fn is_pong(&self) -> bool {
        matches!(*self, Message::Pong(_))
    }

    // pub fn is_close(&self) -> bool {
    //     matches!(*self, Message::Close(_))
    // }

    // pub fn len(&self) -> usize {
    //     match *self {
    //         Message::Text(ref string) => string.len(),
    //         Message::Binary(ref data) | Message::Ping(ref data) | Message::Pong(ref data) => {
    //             data.len()
    //         }
    //         Message::Close(ref data) => data.as_ref().map(|d| d.reason.len()).unwrap_or(0),
    //         Message::Frame(ref frame) => frame.len(),
    //     }
    // }

    // pub fn is_empty(&self) -> bool {
    //     self.len() == 0
    // }

    // pub fn into_data(self) -> Vec<u8> {
    //     match self {
    //         Message::Text(string) => string.into_bytes(),
    //         Message::Binary(data) | Message::Ping(data) | Message::Pong(data) => data,
    //         Message::Close(None) => Vec::new(),
    //         Message::Close(Some(frame)) => frame.reason.into_owned().into_bytes(),
    //         Message::Frame(frame) => frame.into_data(),
    //     }
    // }

    // pub fn into_text(self) -> Result<String> {
    //     match self {
    //         Message::Text(string) => Ok(string),
    //         Message::Binary(data) | Message::Ping(data) | Message::Pong(data) => {
    //             Ok(String::from_utf8(data)?)
    //         }
    //         Message::Close(None) => Ok(String::new()),
    //         Message::Close(Some(frame)) => Ok(frame.reason.into_owned()),
    //         Message::Frame(frame) => Ok(frame.into_string()?),
    //     }
    // }

    // pub fn to_text(&self) -> Result<&str> {
    //     match *self {
    //         Message::Text(ref string) => Ok(string),
    //         Message::Binary(ref data) | Message::Ping(ref data) | Message::Pong(ref data) => {
    //             Ok(str::from_utf8(data)?)
    //         }
    //         Message::Close(None) => Ok(""),
    //         Message::Close(Some(ref frame)) => Ok(&frame.reason),
    //         Message::Frame(ref frame) => Ok(frame.to_text()?),
    //     }
    // }
}

impl From<String> for Message {
    fn from(string: String) -> Self {
        Message::text(string)
    }
}

impl<'s> From<&'s str> for Message {
    fn from(string: &'s str) -> Self {
        Message::text(string)
    }
}

impl<'b> From<&'b [u8]> for Message {
    fn from(data: &'b [u8]) -> Self {
        Message::binary(data)
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        Message::binary(data)
    }
}

// impl From<Message> for Vec<u8> {
//     fn from(message: Message) -> Self {
//         message.into_data()
//     }
// }

// impl TryFrom<Message> for String {
//     type Error = Error;
//
//     fn try_from(value: Message) -> StdResult<Self, Self::Error> {
//         value.into_text()
//     }
// }

// impl fmt::Display for Message {
//     fn fmt(&self, f: &mut fmt::Formatter) -> StdResult<(), fmt::Error> {
//         if let Ok(string) = self.to_text() {
//             write!(f, "{}", string)
//         } else {
//             write!(f, "Binary Data<length={}>", self.len())
//         }
//     }
// }

////////

pub struct Sender {

}

pub struct Error {

}

pub struct Token {

}

pub struct Frame {

}

pub struct Timeout {

}

pub trait Factory {
    type Handler: Handler;

    fn connection_made(&mut self, _: Sender) -> Self::Handler;

    #[inline]
    fn on_shutdown(&mut self) {
    
    }

    #[inline]
    fn client_connected(&mut self, ws: Sender) -> Self::Handler {
        self.connection_made(ws)
    }

    #[inline]
    fn server_connected(&mut self, ws: Sender) -> Self::Handler {
        self.connection_made(ws)
    }

    #[inline]
    fn connection_lost(&mut self, _: Self::Handler) {}
}

impl<F, H> Factory for F
where
    H: Handler,
    F: FnMut(Sender) -> H,
{
    type Handler = H;

    fn connection_made(&mut self, out: Sender) -> H {
        self(out)
    }
}

pub trait Handler {
    #[inline]
    fn on_shutdown(&mut self) {
        // debug!("Handler received WebSocket shutdown request.");
    }

    fn on_open(&mut self) -> Result<()> {
        // if let Some(addr) = shake.remote_addr()? {
        //     // debug!("Connection with {} now open", addr);
        // }
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        // debug!("Received message {:?}", msg);
        Ok(())
    }

    fn on_close(&mut self, code: Close_Code, reason: &str) {
        // debug!("Connection closing due to ({:?}) {}", code, reason);
    }

    fn on_error(&mut self, err: Error) {
        // Ignore connection reset errors by default, but allow library clients to see them by
        // overriding this method if they want
        // if let Kind::Io(ref err) = err.kind {
        //     if let Some(104) = err.raw_os_error() {
        //         return;
        //     }
        // }
        //
        // error!("{:?}", err);
        // if !log_enabled!(ErrorLevel) {
        //     println!(
        //         "Encountered an error: {}\nEnable a logger to see more information.",
        //         err
        //     );
        // }
    }

    #[inline]
    fn on_timeout(&mut self, event: Token) -> Result<()> {
        // debug!("Handler received timeout token: {:?}", event);
        Ok(())
    }

    #[inline]
    fn on_new_timeout(&mut self, _: Token, _: Timeout) -> Result<()> {
        // default implementation discards the timeout handle
        Ok(())
    }

    #[inline]
    fn on_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        // debug!("Handler received: {}", frame);
        // default implementation doesn't allow for reserved bits to be set
        // if frame.has_rsv1() || frame.has_rsv2() || frame.has_rsv3() {
            // Err(Error::new(
            //     Kind::Protocol,
            //     "Encountered frame with reserved bits set.",
            // ))
        // } else {
            Ok(Some(frame))
        // }
    }

    #[inline]
    fn on_send_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        // trace!("Handler will send: {}", frame);
        // default implementation doesn't allow for reserved bits to be set
        // if frame.has_rsv1() || frame.has_rsv2() || frame.has_rsv3() {
            // Err(Error::new(
            //     Kind::Protocol,
            //     "Encountered frame with reserved bits set.",
            // ))
        // } else {
            Ok(Some(frame))
        // }
    }

    // #[inline]
    // fn build_request(&mut self, url: &url::Url) -> Result<Request> {
    //     // trace!("Handler is building request to {}.", url);
    //     Request::from_url(url)
    // }
}

impl<F> Handler for F
where
    F: Fn(Message) -> Result<()>,
{
    fn on_message(&mut self, msg: Message) -> Result<()> {
        self(msg)
    }
}

pub struct WebSocket {
}

impl Service for WebSocket {
    fn uri(&self, request: &Request) -> bool {
        if let Some(header) = request.headers.get("Connection") {
            header == "Upgrade"
        } else {
            false
        }
    }

    // async fn serve(&self) {
    //
    // }

    // fn respond(request: &Request, resources: Arc<Mutex<Resources>>) -> Response {
    //     let key = request.headers.get("Sec-WebSocket-Key").unwrap();
    //     println!("websocket key : {key}");
    //
    //     let hashed_key = hash_key(key);
    //
    //     let mut headers: HashMap<String, String> = HashMap::new();
    //     headers.insert("Connection".into(), "Upgrade".into());
    //     headers.insert("Upgrade".into(), "websocket".into());
    //     headers.insert("Sec-WebSocket-Accept".into(), hashed_key);
    //
    //     Response {
    //         status: Status_Code::SwitchingProtocols,
    //         headers,
    //         body: Vec::new(),
    //     }
    // }

    fn handler(&self, stream: &mut TcpStream, resources: Arc<Mutex<Resources>>) {
        let mut client = accept(stream).unwrap();
        loop {
            let message = client.read();
            // println!("{:?}", msg);
            // We do not want to send back ping/pong messages.

            // if msg.is_close() {
            //     println!("close");
            // }
            // else if msg.is_empty() {
            //     println!("open?");
            // }
            // else if msg.is_binary() || msg.is_text() {
            //     // // websocket.send(msg).unwrap();
            //     //
            //     // let json : JSON = serde_json::from_str(&msg.to_string()).unwrap();
            //     // let message_type = Message_Type::from_str(json["#type"].clone().as_str().unwrap()).unwrap();
            //     // println!("message type is : {:?}", message_type);
            // }
        }
    }
}

impl WebSocket {
    pub fn read( &self ) -> Result<Message> {
        Ok(Message::Text("hello world".into()))
    }

    pub fn on_client_connected() {

    }
}

struct Connection<'a> {
    stream: &'a mut TcpStream
}

impl<'a> Connection<'a> {
    pub async fn read( &mut self ) -> Option<Message> {

        let mut buffer = [0_u8; 512];

        self.stream.read(&mut buffer).await.unwrap();

        if buffer.is_empty() {
            return None
        }

        // let marker_and_payload_length = self.read_stream(1)[0];
        //
        // let length_indicator_in_bits = marker_and_payload_length - FIRST_BIT;
        //
        // let mut message_length = 0;
        // if length_indicator_in_bits <= SEVEN_BITS_INTEGER_MARKER {
        //     message_length = length_indicator_in_bits;
        // } else if length_indicator_in_bits == SIXTEEN_BITS_INTEGER_MARKER {
        //     message_length = buffer[2];
        // }

        println!("got message");
        // let message = String::from_utf8_lossy(&buffer);
        // println!("{}", message);

        Some(Message::Text("Hello vasya".to_string()))
    }

    async fn read_stream( &mut self, size: u8 ) -> Vec<u8> {
        let mut buffer = [0; 1]; 
        self.stream.read_exact( &mut buffer ).await.unwrap();
        buffer.to_vec()
    }
}


fn accept( stream: &mut TcpStream ) -> Result<Connection> {
    Ok(Connection {
        stream
    }) 
}

pub fn handle_connection( stream: &mut TcpStream, _: Arc<Mutex<Resources>> ) {

    let mut client = accept(stream).unwrap();
    loop {
        let message = client.read();
        // println!("{:?}", msg);
        // We do not want to send back ping/pong messages.

        // if msg.is_close() {
        //     println!("close");
        // }
        // else if msg.is_empty() {
        //     println!("open?");
        // }
        // else if msg.is_binary() || msg.is_text() {
        //     // // websocket.send(msg).unwrap();
        //     //
        //     // let json : JSON = serde_json::from_str(&msg.to_string()).unwrap();
        //     // let message_type = Message_Type::from_str(json["#type"].clone().as_str().unwrap()).unwrap();
        //     // println!("message type is : {:?}", message_type);
        // }
    }
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
