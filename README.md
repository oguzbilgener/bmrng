# bmrng

![Crates.io](https://img.shields.io/crates/v/bmrng)
![Documentation](https://docs.rs/bmrng/badge.svg)
![Unit Tests](https://github.com/oguzbilgener/bmrng/workflows/Unit%20Tests/badge.svg)
[![codecov](https://codecov.io/gh/oguzbilgener/bmrng/branch/master/graph/badge.svg?token=8V51592OVH)](https://codecov.io/gh/oguzbilgener/bmrng)
[![Dependency status](https://deps.rs/repo/github/oguzbilgener/bmrng/status.svg)](https://deps.rs/repo/github/oguzbilgener/bmrng/status.svg)

An async MPSC request-response channel for Tokio, where you can send a response to the sender.
Inspired by [crossbeam_requests](https://docs.rs/crate/crossbeam_requests).