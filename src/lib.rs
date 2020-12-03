#![forbid(unsafe_code)]

mod bounded;
pub use self::bounded::{
    channel, Payload, RequestReceiver, RequestSender, Responder, ResponseReceiver,
};
pub mod error;
pub mod unbounded;
pub use unbounded::channel as unbounded_channel;
