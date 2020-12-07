use bmrng::error::*;
use tokio::time::{advance, pause, resume, sleep, Duration};
use tokio_test::assert_ok;

#[tokio::test]
async fn unbounded_send_receive() {
    let (tx, mut rx) = bmrng::unbounded_channel::<i32, i32>();
    tokio::spawn(async move {
        match rx.recv().await {
            Ok((input, mut responder)) => {
                assert_eq!(responder.is_closed(), false);
                let res = responder.respond(input * input);
                assert_eq!(responder.is_closed(), true);
                assert_ok!(res);
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    assert_eq!(tx.is_closed(), false);
    let response = tx.send_receive(8).await;
    assert_eq!(tx.is_closed(), true);
    assert_eq!(response.is_ok(), true);
    assert_eq!(response.unwrap(), 64);
}

#[tokio::test]
async fn bounded_send_receive() {
    let (tx, mut rx) = bmrng::channel::<i32, i32>(1);
    tokio::spawn(async move {
        match rx.recv().await {
            Ok((input, mut responder)) => {
                assert_eq!(responder.is_closed(), false);
                let res = responder.respond(input * input);
                assert_eq!(responder.is_closed(), true);
                assert_ok!(res);
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    assert_eq!(tx.is_closed(), false);
    let response = tx.send_receive(8).await;
    assert_eq!(tx.is_closed(), true);
    assert_eq!(response.is_ok(), true);
    assert_eq!(response.unwrap(), 64);
}

#[tokio::test]
async fn unbounded_drop_while_waiting_for_response() {
    let (tx, mut rx) = bmrng::unbounded_channel::<i32, i32>();
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok((_, responder)) => {
                drop(responder);
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response = tx.send_receive(8).await;
    assert_ok!(tokio::join!(task).0);
    assert_eq!(response, Err(RequestError::RecvError));
}

#[tokio::test]
async fn unbounded_drop_while_waiting_for_request() {
    let (tx, mut rx) = bmrng::unbounded_channel::<i32, i32>();
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok(_) => panic!("this should not be ok"),
            Err(_) => {}
        };
    });
    drop(tx);
    assert_ok!(tokio::join!(task).0);
}

#[tokio::test]
async fn unbounded_drop_sender_while_sending_response() {
    let (tx, mut rx) = bmrng::unbounded_channel::<i32, i32>();
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok((_, mut responder)) => {
                let respond_result  = responder.respond(42);
                assert_eq!(respond_result, Err(RespondError::ChannelClosed(42)));
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response_receiver = tx.send(21);
    drop(response_receiver);
    assert_ok!(tokio::join!(task).0);
}

#[tokio::test]
async fn bounded_drop_while_waiting_for_response() {
    let (tx, mut rx) = bmrng::channel::<i32, i32>(1);
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok((_, responder)) => {
                drop(responder);
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response = tx.send_receive(8).await;
    assert_ok!(tokio::join!(task).0);
    assert_eq!(response, Err(RequestError::RecvError));
}

#[tokio::test]
async fn bounded_drop_while_waiting_for_request() {
    let (tx, mut rx) = bmrng::channel::<i32, i32>(1);
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok(_) => panic!("this should not be ok"),
            Err(_) => {}
        };
    });
    drop(tx);
    assert_ok!(tokio::join!(task).0);
}

#[tokio::test]
async fn bounded_drop_sender_while_sending_response() {
    let (tx, mut rx) = bmrng::channel::<i32, i32>(1);
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok((_, mut responder)) => {
                let respond_result  = responder.respond(42);
                assert_eq!(respond_result, Err(RespondError::ChannelClosed(42)));
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response_receiver = tx.send(21).await;
    drop(response_receiver);
    assert_ok!(tokio::join!(task).0);
}

#[tokio::test]
async fn unbounded_already_replied() {
    let (tx, mut rx) = bmrng::unbounded_channel::<i32, i32>();
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok((data, mut responder)) => {
                let result = responder.respond(data - 2);
                assert_ok!(result);
                let result2 = responder.respond(data - 3);
                assert_eq!(result2, Err(RespondError::AlreadyReplied(5)));
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response = tx.send_receive(8).await;
    assert_ok!(tokio::join!(task).0);
    assert_eq!(response, Ok(6));
}

#[tokio::test]
async fn bounded_already_replied() {
    let (tx, mut rx) = bmrng::channel::<i32, i32>(1);
    let task = tokio::spawn(async move {
        match rx.recv().await {
            Ok((data, mut responder)) => {
                let result = responder.respond(data - 2);
                assert_ok!(result);
                let result2 = responder.respond(data - 3);
                assert_eq!(result2, Err(RespondError::AlreadyReplied(5)));
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response = tx.send_receive(8).await;
    assert_ok!(tokio::join!(task).0);
    assert_eq!(response, Ok(6));
}

#[tokio::test]
async fn bounded_timeout() {
    let (tx, mut rx) = bmrng::channel_with_timeout::<i32, i32>(1, Duration::from_millis(100));
    pause();
    tokio::spawn(async move {
        match rx.recv().await {
            Ok((_input, responder)) => {
                assert_eq!(responder.is_closed(), false);
                advance(Duration::from_millis(200)).await;
                sleep(Duration::from_micros(1)).await;
                resume();
                panic!("Should have timed out");
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    assert_eq!(tx.is_closed(), false);
    let response = tx.send_receive(8).await;
    assert_eq!(response, Err(RequestError::<i32>::RecvTimeoutError));
}

#[tokio::test]
async fn unbounded_timeout() {
    let (tx, mut rx) =
        bmrng::unbounded::channel_with_timeout::<i32, i32>(Duration::from_millis(100));
    pause();
    tokio::spawn(async move {
        match rx.recv().await {
            Ok((_input, responder)) => {
                assert_eq!(responder.is_closed(), false);
                advance(Duration::from_millis(200)).await;
                sleep(Duration::from_micros(1)).await;
                resume();
                panic!("Should have timed out");
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    assert_eq!(tx.is_closed(), false);
    let response = tx.send_receive(8).await;
    assert_eq!(response, Err(RequestError::<i32>::RecvTimeoutError));
}
