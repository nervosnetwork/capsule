#!/bin/bash

echo "Prepare test environment..."
rm -rf tmp/rust-demo/

echo "Running tests..."
cargo run -p tests

echo "Ok"
