use crate::error::{RequestError, RespondError, SendError};

use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// The internal data sent in the MPSC request channel, a tuple that contains the request and the oneshot response channel responder
pub type Payload<Req, Res> = (Req, Responder<Res>);

/// Send values to the associated [`RequestReceiver`].
#[derive(Debug)]
pub struct RequestSender<Req, Res> {
    request_sender: mpsc::Sender<Payload<Req, Res>>,
}

/// Receive requests values from the associated [`RequestSender`]
///
/// Instances are created by the [`channel`] function.
#[derive(Debug)]
pub struct RequestReceiver<Req, Res> {
    request_receiver: mpsc::Receiver<Payload<Req, Res>>,
}

/// Send values back to the [`RequestSender`] or [`RequestReceiver`]
///
/// Instances are created by calling [`RequestSender::send_receive()`] or [`RequestSender::send()`]
#[derive(Debug)]
pub struct Responder<Res> {
    response_sender: Option<oneshot::Sender<Res>>,
}

/// Receive responses from a [`Responder`]
///
/// Instances are created by calling [`RequestSender::send_receive()`] or [`RequestSender::send()`]
#[derive(Debug)]
pub struct ResponseReceiver<Res> {
    response_receiver: Option<oneshot::Receiver<Res>>,
}

impl<Req, Res> RequestSender<Req, Res> {
    fn new(request_sender: mpsc::Sender<Payload<Req, Res>>) -> Self {
        RequestSender { request_sender }
    }

    /// Send a request over the MPSC channel, open the response channel
    /// Return the [`ResponseReceiver`] which can be used to wait for a response
    ///
    /// This call blocks if the request channel is full. It does not wait for a response
    pub async fn send(&self, request: Req) -> Result<ResponseReceiver<Res>, SendError<Req>> {
        let (response_sender, response_receiver) = oneshot::channel::<Res>();
        let responder = Responder::new(response_sender);
        let payload = (request, responder);
        self.request_sender
            .send(payload)
            .await
            .map_err(|payload| SendError(payload.0 .0))?;
        let receiver = ResponseReceiver::new(response_receiver);
        Ok(receiver)
    }

    /// Send a request over the MPSC channel, wait for the response and return it
    ///
    /// This call blocks if the request channel is full, and while waiting for the response
    pub async fn send_receive(&self, request: Req) -> Result<Res, RequestError<Req>> {
        let mut receiver = self.send(request).await?;
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

impl<Req, Res> RequestReceiver<Req, Res> {
    fn new(receiver: mpsc::Receiver<Payload<Req, Res>>) -> Self {
        RequestReceiver {
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

impl<Res> Responder<Res> {
    fn new(response_sender: oneshot::Sender<Res>) -> Self {
        Self {
            response_sender: Some(response_sender),
        }
    }

    /// Responds a request from the [`RequestSender`] which finishes the request
    pub fn respond(&mut self, response: Res) -> Result<(), RespondError<Res>> {
        match self.response_sender.take() {
            Some(response_sender) => response_sender
                .send(response)
                .map_err(|res| RespondError::ChannelClosed(res)),
            None => Err(RespondError::AlreadyReplied(response)),
        }
    }
}

/// Creates a bounded mpsc request-response  channel for communicating between
/// asynchronous tasks with backpressure
///
/// # Panics
///
/// Panics if the buffer capacity is 0, just like the Tokio MPSC channel
///
/// # Examples
///
/// ```rust
/// #[tokio::main]
/// async fn main() {
///     let (tx, mut rx) = bmrng::channel::<i32, i32>(100);
///     tokio::spawn(async move {
///         match rx.recv().await {
///             Ok((input, mut responder)) => {
///                 let res = responder.respond(input * input);
///                 assert_eq!(res.is_ok(), true);
///             }
///             Err(err) => {
///                 panic!(err);
///             }
///         }
///     });
///     let response = tx.send_receive(8).await;
///     assert_eq!(response.unwrap(), 64);
/// }
/// ```
pub fn channel<Req, Res>(buffer: usize) -> (RequestSender<Req, Res>, RequestReceiver<Req, Res>) {
    let (sender, receiver) = mpsc::channel::<Payload<Req, Res>>(buffer);
    let request_sender = RequestSender::new(sender);
    let request_receiver = RequestReceiver::new(receiver);
    (request_sender, request_receiver)
}
