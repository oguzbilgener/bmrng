use crate::error::{RequestError, RespondError, SendError};

use crate::bounded::{ResponseReceiver, Request};
use tokio::sync::{mpsc, oneshot};
use tokio::time::Duration;

use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// The internal data sent in the MPSC request channel, a tuple that contains the request and the oneshot response channel responder
pub type Payload<R: Request> = (R, UnboundedResponder<R::Response>);

/// Send values to the associated [`UnboundedRequestReceiver`].
#[derive(Debug)]
pub struct UnboundedRequestSender<R: Request> {
    request_sender: mpsc::UnboundedSender<Payload<R>>,
    timeout_duration: Option<Duration>,
}

/// Receive requests values from the associated [`UnboundedRequestSender`]
///
/// Instances are created by the [`channel`] function.
#[derive(Debug)]
pub struct UnboundedRequestReceiver<R: Request> {
    request_receiver: mpsc::UnboundedReceiver<Payload<R>>,
}

/// Send values back to the [`UnboundedRequestSender`] or [`UnboundedRequestReceiver`]
///
/// Instances are created by calling [`UnboundedRequestSender::send_receive()`] or [`UnboundedRequestSender::send()`]
#[derive(Debug)]
pub struct UnboundedResponder<R: Request> {
    response_sender: oneshot::Sender<R::Response>,
}

impl<R: Request> UnboundedRequestSender<R> {
    fn new(
        request_sender: mpsc::UnboundedSender<Payload<R>>,
        timeout_duration: Option<Duration>,
    ) -> Self {
        UnboundedRequestSender {
            request_sender,
            timeout_duration,
        }
    }

    /// Send a request over the MPSC channel, open the response channel
    /// Return the [`ResponseReceiver`] which can be used to wait for a response
    pub fn send(&self, request: R) -> Result<ResponseReceiver<R::Response>, SendError<R>> {
        let (response_sender, response_receiver) = oneshot::channel::<R::Response>();
        let responder = UnboundedResponder::new(response_sender);
        let payload = (request, responder);
        self.request_sender
            .send(payload)
            .map_err(|payload| SendError(payload.0 .0))?;
        let receiver = ResponseReceiver::new(response_receiver, self.timeout_duration);
        Ok(receiver)
    }

    /// Send a request over the MPSC channel, wait for the response and return it
    pub async fn send_receive(&self, request: R) -> Result<R::Response, RequestError<R>> {
        let mut receiver = self.send(request)?;
        receiver.recv().await.map_err(|err| err.into())
    }

    /// Checks if the channel has been closed.
    pub fn is_closed(&self) -> bool {
        self.request_sender.is_closed()
    }
}

impl<R: Request> Clone for UnboundedRequestSender<R> {
    fn clone(&self) -> Self {
        UnboundedRequestSender {
            request_sender: self.request_sender.clone(),
            timeout_duration: self.timeout_duration,
        }
    }
}

impl<R: Request> UnboundedRequestReceiver<R> {
    fn new(receiver: mpsc::UnboundedReceiver<Payload<R>>) -> Self {
        UnboundedRequestReceiver {
            request_receiver: receiver,
        }
    }

    /// Receives the next value for this receiver.
    pub async fn recv(&mut self) -> Result<Payload<R>, RequestError<R>> {
        match self.request_receiver.recv().await {
            Some(payload) => Ok(payload),
            None => Err(RequestError::RecvError),
        }
    }

    /// Closes the receiving half of a channel without dropping it.
    pub fn close(&mut self) {
        self.request_receiver.close()
    }

    /// Converts this receiver into a stream
    pub fn into_stream(self) -> impl Stream<Item = Payload<R>> {
        let stream: UnboundedRequestReceiverStream<R> = self.into();
        stream
    }
}

impl<R: Request> UnboundedResponder<R::Response> {
    fn new(response_sender: oneshot::Sender<R::Response>) -> Self {
        Self { response_sender }
    }

    /// Responds a request from the [`UnboundedRequestSender`] which finishes the request
    pub fn respond(self, response: R::Response) -> Result<(), RespondError<R::Response>> {
        self.response_sender.send(response).map_err(RespondError)
    }

    /// Checks if the associated receiver handle for the response listener has been dropped.
    pub fn is_closed(&self) -> bool {
        self.response_sender.is_closed()
    }
}

/// Creates an unbounded mpsc request-response channel for communicating between
/// asynchronous tasks without backpressure.
///
/// Also see [`bmrng::channel()`](crate::bounded::channel)
pub fn channel<R: Request>() -> (
    UnboundedRequestSender<R>,
    UnboundedRequestReceiver<R>,
) {
    let (sender, receiver) = mpsc::unbounded_channel::<Payload<R>>();
    let request_sender = UnboundedRequestSender::new(sender, None);
    let request_receiver = UnboundedRequestReceiver::new(receiver);
    (request_sender, request_receiver)
}

/// Creates an unbounded mpsc request-response channel for communicating between
/// asynchronous tasks without backpressure and a request timeout
///
/// Also see [`bmrng::channel()`](crate::bounded::channel())
pub fn channel_with_timeout<R: Request>(
    timeout_duration: Duration,
) -> (
    UnboundedRequestSender<R>,
    UnboundedRequestReceiver<R>,
) {
    let (sender, receiver) = mpsc::unbounded_channel::<Payload<R>>();
    let request_sender = UnboundedRequestSender::new(sender, Some(timeout_duration));
    let request_receiver = UnboundedRequestReceiver::new(receiver);
    (request_sender, request_receiver)
}

/// A wrapper around [`UnboundedRequestReceiver`] that implements [`Stream`].
#[derive(Debug)]
pub struct UnboundedRequestReceiverStream<R: Request> {
    inner: UnboundedRequestReceiver<R>,
}

impl<R: Request> UnboundedRequestReceiverStream<R> {
    /// Create a new `RequestReceiverStream`.
    pub fn new(recv: UnboundedRequestReceiver<R>) -> Self {
        Self { inner: recv }
    }

    /// Get back the inner `Receiver`.
    #[cfg(not(tarpaulin_include))]
    pub fn into_inner(self) -> UnboundedRequestReceiver<R> {
        self.inner
    }

    /// Closes the receiving half of a channel without dropping it.
    #[cfg(not(tarpaulin_include))]
    pub fn close(&mut self) {
        self.inner.close()
    }
}

impl<R: Request> Stream for UnboundedRequestReceiverStream<R> {
    type Item = Payload<R>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.request_receiver.poll_recv(cx)
    }
}

impl<R: Request> AsRef<UnboundedRequestReceiver<R>>
    for UnboundedRequestReceiverStream<R>
{
    #[cfg(not(tarpaulin_include))]
    fn as_ref(&self) -> &UnboundedRequestReceiver<R> {
        &self.inner
    }
}

impl<R: Request> AsMut<UnboundedRequestReceiver<R>>
    for UnboundedRequestReceiverStream<R>
{
    #[cfg(not(tarpaulin_include))]
    fn as_mut(&mut self) -> &mut UnboundedRequestReceiver<R> {
        &mut self.inner
    }
}

impl<R: Request> From<UnboundedRequestReceiver<R>>
    for UnboundedRequestReceiverStream<R>
{
    fn from(receiver: UnboundedRequestReceiver<R>) -> Self {
        UnboundedRequestReceiverStream::new(receiver)
    }
}
