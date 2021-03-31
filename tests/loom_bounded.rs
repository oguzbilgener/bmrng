use loom::future::block_on;
use loom::thread;
use tokio_test::assert_err;
use tokio_test::assert_ok;

#[test]
#[cfg(not(tarpaulin))]
fn closing_tx() {
    loom::model(|| {
        let (tx, mut rx) = bmrng::channel::<u32, u32>(16);

        thread::spawn(move || {
            let res = block_on(tx.send(4));
            assert_ok!(res);
            drop(tx);
        });

        let v = block_on(rx.recv());
        assert_ok!(v);

        let v = block_on(rx.recv());
        assert_err!(v);
    })
}

#[test]
#[cfg(not(tarpaulin))]
fn closing_tx_res() {
    loom::model(|| {
        let (tx, mut rx) = bmrng::channel::<u32, u32>(16);

        thread::spawn(move || {
            let res = block_on(tx.send(5));
            let repl = block_on(res.unwrap().recv());
            assert_ok!(repl);
            assert_eq!(repl.unwrap(), 10);
            drop(tx);
        });

        let v = block_on(rx.recv());
        let (req, responder) = v.unwrap();
        let v = responder.respond(req * 2);
        assert_ok!(v);

        let v = block_on(rx.recv());
        assert_err!(v);
    })
}
