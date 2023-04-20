#!/bin/bash

echo "Prepare test environment..."
rm -rf tmp/rust-demo/

echo "Build..."
cargo build

echo "Running tests..."
cargo run -p tests

echo "Ok"
