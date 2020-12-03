use std::error::Error;
use std::fmt;
use tokio::sync::mpsc::error::{RecvError as MpscRecvError, SendError as MpscSendError};
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct SendError<T>(pub T);

impl<T> From<SendError<T>> for MpscSendError<T> {
    fn from(err: SendError<T>) -> Self {
        Self(err.0)
    }
}

impl<T> From<MpscSendError<T>> for SendError<T> {
    fn from(err: MpscSendError<T>) -> Self {
        Self(err.0)
    }
}

impl<T> fmt::Display for SendError<T> {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "channel closed")
    }
}

impl<T> Error for SendError<T> where T: fmt::Debug {}

#[derive(Debug, Copy, Clone, PartialEq)]
/// Errors that can occur when a [RequestReceiver] handles a request
pub enum RequestError<T> {
    /// Error occurring when the channel from [RequestSender] to [RequestReceiver] is closed
    RecvError,
    /// Error occurring when the channel from [RequestReceiver] to [RequestSender] is closed
    SendError(T),
}

// Cannot test this due to private field in the tokio error implementation
#[cfg(not(tarpaulin_include))]
impl<T> From<MpscRecvError> for RequestError<T> {
    fn from(_err: MpscRecvError) -> RequestError<T> {
        RequestError::RecvError
    }
}
impl<T> From<SendError<T>> for RequestError<T> {
    fn from(err: SendError<T>) -> RequestError<T> {
        RequestError::SendError(err.0)
    }
}

impl<T> From<ReplyError<T>> for RequestError<T> {
    fn from(err: ReplyError<T>) -> RequestError<T> {
        match err {
            ReplyError::AlreadyReplied(item) => RequestError::SendError(item),
            ReplyError::ChannelClosed(item) => RequestError::SendError(item),
        }
    }
}

impl<T> From<oneshot::error::RecvError> for RequestError<T> {
    fn from(_err: oneshot::error::RecvError) -> RequestError<T> {
        RequestError::RecvError
    }
}

impl<T> fmt::Display for RequestError<T> {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                RequestError::RecvError => "request channel closed",
                RequestError::SendError(..) => "channel closed",
            }
        )
    }
}

impl<T> Error for RequestError<T> where T: fmt::Debug {}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReplyError<T> {
    AlreadyReplied(T),
    ChannelClosed(T),
}

impl<T> fmt::Display for ReplyError<T> {
    #[cfg(not(tarpaulin_include))]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                ReplyError::AlreadyReplied(..) => "already replied",
                ReplyError::ChannelClosed(..) => "sender closed the reply channel",
            }
        )
    }
}

impl<T> Error for ReplyError<T> where T: fmt::Debug {}

pub mod tests {
    pub use super::*;

    #[test]
    fn send_err_into_mpsc_send_error() {
        let err = SendError(42);
        let m_err: MpscSendError<u32> = err.into();
        assert_eq!(m_err.0, 42);
    }

    #[test]
    fn mpsc_send_err_into_send_error() {
        let err = MpscSendError(42);
        let m_err: SendError<u32> = err.into();
        assert_eq!(m_err.0, 42);
    }

    #[test]
    fn send_error_into_request_error() {
        let err = SendError(42);
        let r_err: RequestError<i32> = err.into();
        assert_eq!(r_err, RequestError::SendError(42));
    }

    #[test]
    fn reply_error_to_request_error() {
        let err = ReplyError::AlreadyReplied(21);
        let q_err: RequestError<i32> = err.into();
        assert_eq!(q_err, RequestError::SendError(21));
        let err = ReplyError::ChannelClosed(21);
        let q_err: RequestError<i32> = err.into();
        assert_eq!(q_err, RequestError::SendError(21));
    }
}
