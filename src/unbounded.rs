use crate::error::{RequestError, RespondError, SendError};

use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type Payload<Req, Res> = (Req, UnboundedResponder<Res>);

#[derive(Debug)]
pub struct UnboundedRequestSender<Req, Res> {
    request_sender: mpsc::UnboundedSender<Payload<Req, Res>>,
}

#[derive(Debug)]
pub struct UnboundedRequestReceiver<Req, Res> {
    request_receiver: mpsc::UnboundedReceiver<Payload<Req, Res>>,
}

#[derive(Debug)]
pub struct UnboundedResponder<Res> {
    response_sender: Option<oneshot::Sender<Res>>,
}

#[derive(Debug)]
pub struct ResponseReceiver<Res> {
    response_receiver: Option<oneshot::Receiver<Res>>,
}

impl<Req, Res> UnboundedRequestSender<Req, Res> {
    fn new(request_sender: mpsc::UnboundedSender<Payload<Req, Res>>) -> Self {
        UnboundedRequestSender { request_sender }
    }

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

    pub async fn send_receive(&self, request: Req) -> Result<Res, RequestError<Req>> {
        let mut receiver = self.send(request)?;
        receiver
            .recv()
            .await
            .map_err(|_err| RequestError::RecvError)
    }
}

impl<Req, Res> UnboundedRequestReceiver<Req, Res> {
    fn new(receiver: mpsc::UnboundedReceiver<Payload<Req, Res>>) -> Self {
        UnboundedRequestReceiver {
            request_receiver: receiver,
        }
    }
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

    /// Responds a request from the [RequestSender] which finishes the request
    pub fn respond(&mut self, response: Res) -> Result<(), RespondError<Res>> {
        match self.response_sender.take() {
            Some(response_sender) => response_sender
                .send(response)
                .map_err(|res| RespondError::ChannelClosed(res)),
            None => Err(RespondError::AlreadyReplied(response)),
        }
    }
}

pub fn channel<Req, Res>() -> (
    UnboundedRequestSender<Req, Res>,
    UnboundedRequestReceiver<Req, Res>,
) {
    let (sender, receiver) = mpsc::unbounded_channel::<Payload<Req, Res>>();
    let request_sender = UnboundedRequestSender::new(sender);
    let request_receiver = UnboundedRequestReceiver::new(receiver);
    (request_sender, request_receiver)
}
