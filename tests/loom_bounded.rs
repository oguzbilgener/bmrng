use bmrng::{channel, Request, Payload};
use loom::future::block_on;
use loom::thread;

#[derive(Debug)]
struct Req(u32);

impl Request for Req {
    type Response = u32;
}

#[test]
#[cfg(not(tarpaulin))]
fn closing_tx() {
    loom::model(|| {
        let (tx, mut rx) = channel::<Req>(16);

        thread::spawn(move || {
            let res = block_on(tx.send(Req(4)));
            assert!(res.is_ok());
            drop(tx);
        });

        let v = block_on(rx.recv());
        assert!(v.is_ok());

        let v = block_on(rx.recv());
        assert!(v.is_err());
    })
}

#[test]
#[cfg(not(tarpaulin))]
fn closing_tx_res() {
    loom::model(|| {
        let (tx, mut rx) = channel::<Req>(16);

        thread::spawn(move || {
            let res = block_on(tx.send(Req(5)));
            let repl = block_on(res.unwrap().recv());
            assert_eq!(repl, Ok(10));
            drop(tx);
        });

        let v = block_on(rx.recv());
        let Payload { request, responder } = v.unwrap();
        let v = responder.respond(request.0 * 2);
        assert!(v.is_ok());

        let v = block_on(rx.recv());
        assert!(v.is_err());
    })
}
