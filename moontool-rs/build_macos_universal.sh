#!/bin/bash

set -e
set -x

cargo build --release --target=aarch64-apple-darwin
cargo build --release --target=x86_64-apple-darwin
lipo -create -output universal/radbot target/{aarch64,x86_64}-apple-darwin/release/radbot
