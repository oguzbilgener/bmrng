use crate::error::{ReplyError, RequestError, SendError};

use tokio::sync::mpsc;
use tokio::sync::oneshot;

pub type Payload<Req, Res> = (Req, Responder<Res>);

pub struct RequestSender<Req, Res> {
    request_sender: mpsc::Sender<Payload<Req, Res>>,
}

pub struct RequestReceiver<Req, Res> {
    request_receiver: mpsc::Receiver<Payload<Req, Res>>,
}

pub struct Responder<Res> {
    response_sender: Option<oneshot::Sender<Res>>,
}

pub struct ResponseReceiver<Res> {
    response_receiver: Option<oneshot::Receiver<Res>>,
}

impl<Req, Res> RequestSender<Req, Res> {
    #[inline(always)]
    fn new(request_sender: mpsc::Sender<Payload<Req, Res>>) -> Self {
        RequestSender { request_sender }
    }

    #[inline(always)]
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

    #[inline(always)]
    pub async fn send_receive(&self, request: Req) -> Result<Res, RequestError<Req>> {
        let mut receiver = self.send(request).await?;
        receiver
            .recv()
            .await
            .map_err(|_err| RequestError::RecvError)
    }
}

impl<Req, Res> RequestReceiver<Req, Res> {
    #[inline(always)]
    fn new(receiver: mpsc::Receiver<Payload<Req, Res>>) -> Self {
        RequestReceiver {
            request_receiver: receiver,
        }
    }
    #[inline(always)]
    pub async fn recv(&mut self) -> Result<Payload<Req, Res>, RequestError<Req>> {
        match self.request_receiver.recv().await {
            Some(payload) => Ok(payload),
            None => Err(RequestError::RecvError),
        }
    }
}

impl<Res> ResponseReceiver<Res> {
    #[inline(always)]
    fn new(response_receiver: oneshot::Receiver<Res>) -> Self {
        Self {
            response_receiver: Some(response_receiver),
        }
    }

    #[inline(always)]
    pub async fn recv(&mut self) -> Result<Res, RequestError<()>> {
        match self.response_receiver.take() {
            Some(response_receiver) => Ok(response_receiver.await?),
            None => Err(RequestError::RecvError),
        }
    }
}

impl<Res> Responder<Res> {
    #[inline(always)]
    fn new(response_sender: oneshot::Sender<Res>) -> Self {
        Self {
            response_sender: Some(response_sender),
        }
    }

    /// Responds a request from the [RequestSender] which finishes the request
    #[inline(always)]
    pub fn respond(&mut self, response: Res) -> Result<(), ReplyError<Res>> {
        match self.response_sender.take() {
            Some(response_sender) => response_sender
                .send(response)
                .map_err(|res| ReplyError::ChannelClosed(res)),
            None => Err(ReplyError::AlreadyReplied(response)),
        }
    }
}

#[inline(always)]
pub fn channel<Req, Res>(buffer: usize) -> (RequestSender<Req, Res>, RequestReceiver<Req, Res>) {
    let (sender, receiver) = mpsc::channel::<Payload<Req, Res>>(buffer);
    let request_sender = RequestSender::new(sender);
    let request_receiver = RequestReceiver::new(receiver);
    (request_sender, request_receiver)
}
