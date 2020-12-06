use bmrng::error::*;
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
