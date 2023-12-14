use crate::handshake::{Request, Response};

// #[derive(Sized)]
pub trait Responder {
    fn respond(&self ) -> &Response where Response : Sized;
}

impl Responder for Response {
    // type Body = BoxBody;
    //
    // #[inline]
    // fn respond(self, _: &Request) -> Response {
    //     self
    // }
    // fn respond(&self) -> &Response where Response : Sized {
    fn respond(&self) -> &Response {
        self
    }
}

// impl<R: Responder> Responder for Option<R> {
//     // type Body = EitherBody<R::Body>;
//
//     fn respond_to(self, req: &Request) -> HttpResponse {
//         match self {
//             Some(val) => val.respond(req).map_into_left_body(),
//             None => Response::new(Status_Code::NOT_FOUND).map_into_right_body(),
//         }
//     }
// }

// macro_rules! impl_responder_by_forward_into_base_response {
//     ($res:ty, $body:ty) => {
//         impl Responder for $res {
//             // type Body = $body;
//
//             fn respond(self, _: &Request) -> Response {
//                 let res: Response = self.into();
//                 res.into()
//             }
//         }
//     };
//
//     ($res:ty) => {
//         impl_responder_by_forward_into_base_response!($res, $res);
//     };
// }
//
// impl_responder_by_forward_into_base_response!(&'static [u8]);
// impl_responder_by_forward_into_base_response!(Vec<u8>);
// // impl_responder_by_forward_into_base_response!(Bytes);
// // impl_responder_by_forward_into_base_response!(BytesMut);
//
// impl_responder_by_forward_into_base_response!(&'static str);
// impl_responder_by_forward_into_base_response!(String);
// // impl_responder_by_forward_into_base_response!(bytestring::ByteString);
//
// macro_rules! impl_into_string_responder {
//     ($res:ty) => {
//         impl Responder for $res {
//             // type Body = String;
//
//             fn respond(self, _: &Request) -> Response {
//                 let string: String = self.into();
//                 let res: Response = string.into();
//                 res.into()
//             }
//         }
//     };
// }
//
// impl_into_string_responder!(&'_ String);
// impl_into_string_responder!(Cow<'_, str>);
