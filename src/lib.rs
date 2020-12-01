mod bounded;
pub use self::bounded::{
    channel, Payload, RequestReceiver, RequestSender, Responder, ResponseReceiver,
};
pub mod error;
pub mod unbounded;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
