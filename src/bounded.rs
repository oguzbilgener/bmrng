use crate::{Request, error::{ReceiveError, RequestError, RespondError, SendError}};

use tokio::sync::{mpsc, oneshot};
use tokio::time::{timeout, Duration};

use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// The internal data sent in the MPSC request channel, a tuple that contains the request and the oneshot response channel responder
#[derive(Debug)]
pub struct Payload<R: Request> {
    /// the request
    pub request: R,
    /// the responder
    pub responder: Responder<R::Response>,
}

/// Send values to the associated [`RequestReceiver`].
#[derive(Debug)]
pub struct RequestSender<R: Request> {
    request_sender: mpsc::Sender<Payload<R>>,
    timeout_duration: Option<Duration>,
}

/// Receive requests values from the associated [`RequestSender`]
///
/// Instances are created by the [`channel`] function.
#[derive(Debug)]
pub struct RequestReceiver<R: Request> {
    request_receiver: mpsc::Receiver<Payload<R>>,
}

/// Send values back to the [`RequestSender`] or [`RequestReceiver`]
///
/// Instances are created by calling [`RequestSender::send_receive()`] or [`RequestSender::send()`]
#[derive(Debug)]
pub struct Responder<R> {
    response_sender: oneshot::Sender<R>,
}

/// Receive responses from a [`Responder`]
///
/// Instances are created by calling [`RequestSender::send_receive()`] or [`RequestSender::send()`]
#[derive(Debug)]
pub struct ResponseReceiver<R> {
    pub(crate) response_receiver: Option<oneshot::Receiver<R>>,
    pub(crate) timeout_duration: Option<Duration>,
}

impl<R: Request> RequestSender<R> {
    fn new(
        request_sender: mpsc::Sender<Payload<R>>,
        timeout_duration: Option<Duration>,
    ) -> Self {
        RequestSender {
            request_sender,
            timeout_duration,
        }
    }

    /// Send a request over the MPSC channel, open the response channel
    ///
    /// Return the [`ResponseReceiver`] which can be used to wait for a response
    ///
    /// This call waits if the request channel is full. It does not wait for a response
    pub async fn send(&self, request: R) -> Result<ResponseReceiver<R::Response>, SendError<R>> {
        let (response_sender, response_receiver) = oneshot::channel::<R::Response>();
        let responder = Responder::new(response_sender);
        let payload = Payload { request, responder };
        self.request_sender
            .send(payload)
            .await
            .map_err(|payload| SendError(payload.0.request))?;
        let receiver = ResponseReceiver::new(response_receiver, self.timeout_duration);
        Ok(receiver)
    }

    /// Send a request over the MPSC channel, wait for the response and return it
    ///
    /// This call waits if the request channel is full, and while waiting for the response
    pub async fn send_receive(&self, request: R) -> Result<R::Response, RequestError<R>> {
        let mut receiver = self.send(request).await?;
        receiver.recv().await.map_err(|err| err.into())
    }

    /// Checks if the channel has been closed.
    pub fn is_closed(&self) -> bool {
        self.request_sender.is_closed()
    }
}

impl<R: Request> Clone for RequestSender<R> {
    fn clone(&self) -> Self {
        RequestSender {
            request_sender: self.request_sender.clone(),
            timeout_duration: self.timeout_duration,
        }
    }
}

impl<R: Request> RequestReceiver<R> {
    fn new(receiver: mpsc::Receiver<Payload<R>>) -> Self {
        RequestReceiver {
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
        let stream: RequestReceiverStream<R> = self.into();
        stream
    }
}

impl<R> ResponseReceiver<R> {
    pub(crate) fn new(
        response_receiver: oneshot::Receiver<R>,
        timeout_duration: Option<Duration>,
    ) -> Self {
        Self {
            response_receiver: Some(response_receiver),
            timeout_duration,
        }
    }

    /// Receives the next value for this receiver.
    ///
    /// If there is a `timeout_duration` set, and the sender takes longer than
    /// the timeout_duration to send the response, it aborts waiting and returns
    /// [`ReceiveError::TimeoutError`].
    pub async fn recv(&mut self) -> Result<R, ReceiveError> {
        match self.response_receiver.take() {
            Some(response_receiver) => match self.timeout_duration {
                Some(duration) => match timeout(duration, response_receiver).await {
                    Ok(response_result) => response_result.map_err(|err| err.into()),
                    Err(..) => Err(ReceiveError::TimeoutError),
                },
                None => Ok(response_receiver.await?),
            },
            None => Err(ReceiveError::RecvError),
        }
    }
}

impl<R> Responder<R> {
    pub(crate) fn new(response_sender: oneshot::Sender<R>) -> Self {
        Self { response_sender }
    }

    /// Responds a request from the [`RequestSender`] which finishes the request
    pub fn respond(self, response: R) -> Result<(), RespondError<R>> {
        self.response_sender.send(response).map_err(RespondError)
    }

    /// Checks if the associated receiver handle for the response listener has been dropped.
    pub fn is_closed(&self) -> bool {
        self.response_sender.is_closed()
    }
}

/// Creates a bounded mpsc request-response channel for communicating between
/// asynchronous tasks with backpressure
///
/// # Panics
///
/// Panics if the buffer capacity is 0, just like the Tokio MPSC channel
///
/// # Examples
///
/// ```rust
/// use bmrng::{Request, Payload};
/// 
/// #[derive(Debug)]
/// struct Req(u32);
/// impl Request for Req {
///     type Response = u32;
/// }
/// 
/// #[tokio::main]
/// async fn main() {
///     let buffer_size = 100;
///     let (tx, mut rx) = bmrng::channel::<Req>(buffer_size);
///     tokio::spawn(async move {
///         while let Ok(Payload { request, mut responder }) = rx.recv().await {
///             if let Err(err) = responder.respond(request.0 * 2) {
///                 println!("sender dropped the response channel");
///             }
///         }
///     });
///     for i in 1..=10 {
///         if let Ok(response) = tx.send_receive(Req(i)).await {
///             println!("Requested {}, got {}", i, response);
///             assert_eq!(response, i * 2);
///         }
///     }
/// }
/// ```
pub fn channel<R: Request>(buffer: usize) -> (RequestSender<R>, RequestReceiver<R>) {
    let (sender, receiver) = mpsc::channel::<Payload<R>>(buffer);
    let request_sender = RequestSender::new(sender, None);
    let request_receiver = RequestReceiver::new(receiver);
    (request_sender, request_receiver)
}

/// Creates a bounded mpsc request-response channel for communicating between
/// asynchronous tasks with backpressure and a request timeout
///
/// # Panics
///
/// Panics if the buffer capacity is 0, just like the Tokio MPSC channel
///
/// # Examples
///
/// ```rust
/// use tokio::time::{Duration, sleep};
/// use bmrng::{Request, Payload};
/// 
/// #[derive(Debug, PartialEq)]
/// struct Req(u32);
/// impl Request for Req {
///     type Response = u32;
/// }
/// 
/// #[tokio::main]
/// async fn main() {
///     let (tx, mut rx) = bmrng::channel_with_timeout::<Req>(100, Duration::from_millis(100));
///     tokio::spawn(async move {
///         match rx.recv().await {
///             Ok(Payload { request, mut responder }) => {
///                 sleep(Duration::from_millis(200)).await;
///                 let res = responder.respond(request.0 * 2);
///                 assert_eq!(res.is_ok(), true);
///             }
///             Err(err) => {
///                 println!("all request senders dropped");
///             }
///         }
///     });
///     let response = tx.send_receive(Req(8)).await;
///     assert_eq!(response, Err(bmrng::error::RequestError::<Req>::RecvTimeoutError));
/// }
/// ```
pub fn channel_with_timeout<R: Request>(
    buffer: usize,
    timeout_duration: Duration,
) -> (RequestSender<R>, RequestReceiver<R>) {
    let (sender, receiver) = mpsc::channel::<Payload<R>>(buffer);
    let request_sender = RequestSender::new(sender, Some(timeout_duration));
    let request_receiver = RequestReceiver::new(receiver);
    (request_sender, request_receiver)
}

/// A wrapper around [`RequestReceiver`] that implements [`Stream`].
#[derive(Debug)]
pub struct RequestReceiverStream<R: Request> {
    inner: RequestReceiver<R>,
}

impl<R: Request> RequestReceiverStream<R> {
    /// Create a new `RequestReceiverStream`.
    pub fn new(recv: RequestReceiver<R>) -> Self {
        Self { inner: recv }
    }

    /// Get back the inner `Receiver`.
    #[cfg(not(tarpaulin_include))]
    pub fn into_inner(self) -> RequestReceiver<R> {
        self.inner
    }

    /// Closes the receiving half of a channel without dropping it.
    #[cfg(not(tarpaulin_include))]
    pub fn close(&mut self) {
        self.inner.close()
    }
}

impl<R: Request> Stream for RequestReceiverStream<R> {
    type Item = Payload<R>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.request_receiver.poll_recv(cx)
    }
}

impl<R: Request> AsRef<RequestReceiver<R>> for RequestReceiverStream<R> {
    #[cfg(not(tarpaulin_include))]
    fn as_ref(&self) -> &RequestReceiver<R> {
        &self.inner
    }
}

impl<R: Request> AsMut<RequestReceiver<R>> for RequestReceiverStream<R> {
    #[cfg(not(tarpaulin_include))]
    fn as_mut(&mut self) -> &mut RequestReceiver<R> {
        &mut self.inner
    }
}

impl<R: Request> From<RequestReceiver<R>> for RequestReceiverStream<R> {
    fn from(receiver: RequestReceiver<R>) -> Self {
        RequestReceiverStream::new(receiver)
    }
}
