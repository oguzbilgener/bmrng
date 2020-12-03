#!/usr/bin/env sh
LOOM_LOG=1 \
# LOOM_LOCATION=1 \
# LOOM_CHECKPOINT_INTERVAL=1 \
LOOM_CHECKPOINT_FILE=loom_checkpoint.json \
RUSTFLAGS="--cfg loom --cfg loom_nightly" cargo +nightly test --test loom_bounded # closing_tx