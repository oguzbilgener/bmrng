# bmrng 🪃

[![Crates.io](https://img.shields.io/crates/v/bmrng)](https://crates.io/crates/bmrng)
[![Documentation](https://docs.rs/bmrng/badge.svg)](https://docs.rs/bmrng)
![Unit Tests](https://github.com/oguzbilgener/bmrng/workflows/Unit%20Tests/badge.svg)
[![codecov](https://codecov.io/gh/oguzbilgener/bmrng/branch/master/graph/badge.svg?token=8V51592OVH)](https://codecov.io/gh/oguzbilgener/bmrng)
[![Dependency status](https://deps.rs/repo/github/oguzbilgener/bmrng/status.svg)](https://deps.rs/repo/github/oguzbilgener/bmrng/status.svg)

An async MPSC request-response channel for Tokio, where you can send a response to the sender.
Inspired by [crossbeam_requests](https://docs.rs/crate/crossbeam_requests).

### Example


```rust
#[tokio::main]
async fn main() {
    let buffer_size = 100;
    let (tx, mut rx) = bmrng::channel::<i32, i32>(buffer_size);
    tokio::spawn(async move {
        match rx.recv().await {
            Ok((input, mut responder)) => {
                let res = responder.respond(input * input);
                assert_eq!(res.is_ok(), true);
            }
            Err(err) => {
                panic!(err);
            }
        }
    });
    let response = tx.send_receive(8).await;
    assert_eq!(response.unwrap(), 64);
}
```