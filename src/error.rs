use tokio::sync::mpsc::error::{RecvError as MpscRecvError, SendError as MpscSendError};
use tokio::sync::oneshot;

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

#[derive(Debug, Copy, Clone, PartialEq)]
/// Errors which can occur when a [RequestReceiver] handles a request
pub enum RequestError<T> {
    /// Error occurring when channel from [RequestSender] to [RequestReceiver] is broken
    RecvError,
    /// Error occurring when channel from [RequestReceiver] to [RequestSender] is broken
    SendError(T),
}
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
            ReplyError::ChannelClosed(item) => RequestError::SendError(item)
        }
    }
}

impl<T> From<oneshot::error::RecvError> for RequestError<T> {
    fn from(_err: oneshot::error::RecvError) -> RequestError<T> {
        RequestError::RecvError
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ReplyError<T> {
    AlreadyReplied(T),
    ChannelClosed(T),
}
