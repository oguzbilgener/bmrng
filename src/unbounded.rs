use crate::error::{RequestError, RespondError, SendError};

use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// The internal data sent in the MPSC request channel, a tuple that contains the request and the oneshot response channel responder
pub type Payload<Req, Res> = (Req, UnboundedResponder<Res>);

/// Send values to the associated [`UnboundedRequestReceiver`].
#[derive(Debug)]
pub struct UnboundedRequestSender<Req, Res> {
    request_sender: mpsc::UnboundedSender<Payload<Req, Res>>,
}

/// Receive requests values from the associated [`UnboundedRequestSender`]
///
/// Instances are created by the [`channel`] function.
#[derive(Debug)]
pub struct UnboundedRequestReceiver<Req, Res> {
    request_receiver: mpsc::UnboundedReceiver<Payload<Req, Res>>,
}

/// Send values back to the [`UnboundedRequestSender`] or [`UnboundedRequestReceiver`]
///
/// Instances are created by calling [`UnboundedRequestSender::send_receive()`] or [`UnboundedRequestSender::send()`]
#[derive(Debug)]
pub struct UnboundedResponder<Res> {
    response_sender: Option<oneshot::Sender<Res>>,
}

/// Receive responses from a [`UnboundedResponder`]
///
/// Instances are created by calling [`UnboundedRequestSender::send_receive()`] or [`UnboundedRequestSender::send()`]
#[derive(Debug)]
pub struct ResponseReceiver<Res> {
    response_receiver: Option<oneshot::Receiver<Res>>,
}

impl<Req, Res> UnboundedRequestSender<Req, Res> {
    fn new(request_sender: mpsc::UnboundedSender<Payload<Req, Res>>) -> Self {
        UnboundedRequestSender { request_sender }
    }

    /// Send a request over the MPSC channel, open the response channel
    /// Return the [`ResponseReceiver`] which can be used to wait for a response
    ///
    /// This call blocks if the request channel is full. It does not wait for a response
    pub fn send(&self, request: Req) -> Result<ResponseReceiver<Res>, SendError<Req>> {
        let (response_sender, response_receiver) = oneshot::channel::<Res>();
        let responder = UnboundedResponder::new(response_sender);
        let payload = (request, responder);
        self.request_sender
            .send(payload)
            .map_err(|payload| SendError(payload.0 .0))?;
        let receiver = ResponseReceiver::new(response_receiver);
        Ok(receiver)
    }

    /// Send a request over the MPSC channel, wait for the response and return it
    ///
    /// This call blocks if the request channel is full, and while waiting for the response
    pub async fn send_receive(&self, request: Req) -> Result<Res, RequestError<Req>> {
        let mut receiver = self.send(request)?;
        receiver
            .recv()
            .await
            .map_err(|_err| RequestError::RecvError)
    }

    /// Checks if the channel has been closed.
    pub fn is_closed(&self) -> bool {
        self.request_sender.is_closed()
    }
}

impl<Req, Res> UnboundedRequestReceiver<Req, Res> {
    fn new(receiver: mpsc::UnboundedReceiver<Payload<Req, Res>>) -> Self {
        UnboundedRequestReceiver {
            request_receiver: receiver,
        }
    }
    /// Receives the next value for this receiver.
    pub async fn recv(&mut self) -> Result<Payload<Req, Res>, RequestError<Req>> {
        match self.request_receiver.recv().await {
            Some(payload) => Ok(payload),
            None => Err(RequestError::RecvError),
        }
    }
}

impl<Res> ResponseReceiver<Res> {
    fn new(response_receiver: oneshot::Receiver<Res>) -> Self {
        Self {
            response_receiver: Some(response_receiver),
        }
    }

    /// Receives the next value for this receiver.
    pub async fn recv(&mut self) -> Result<Res, RequestError<()>> {
        match self.response_receiver.take() {
            Some(response_receiver) => Ok(response_receiver.await?),
            None => Err(RequestError::RecvError),
        }
    }
}

impl<Res> UnboundedResponder<Res> {
    fn new(response_sender: oneshot::Sender<Res>) -> Self {
        Self {
            response_sender: Some(response_sender),
        }
    }

    /// Responds a request from the [`UnboundedRequestSender`] which finishes the request
    pub fn respond(&mut self, response: Res) -> Result<(), RespondError<Res>> {
        match self.response_sender.take() {
            Some(response_sender) => response_sender
                .send(response)
                .map_err(|res| RespondError::ChannelClosed(res)),
            None => Err(RespondError::AlreadyReplied(response)),
        }
    }

    /// Checks if a response has already been sent, or the associated receiver
    /// handle for the response listener has been dropped.
    pub fn is_closed(&self) -> bool {
        match &self.response_sender {
            Some(sender) => sender.is_closed(),
            None => false,
        }
    }
}

/// Creates an unbounded mpsc request-response channel for communicating between
/// asynchronous tasks without backpressure.
pub fn channel<Req, Res>() -> (
    UnboundedRequestSender<Req, Res>,
    UnboundedRequestReceiver<Req, Res>,
) {
    let (sender, receiver) = mpsc::unbounded_channel::<Payload<Req, Res>>();
    let request_sender = UnboundedRequestSender::new(sender);
    let request_receiver = UnboundedRequestReceiver::new(receiver);
    (request_sender, request_receiver)
}
