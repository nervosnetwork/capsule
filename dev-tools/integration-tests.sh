#!/bin/bash

prepare_capsule_test() {
    echo "Prepare Capsule testing environment..."
    rm -rf tmp/* && \
    cargo build

}

prepare_testtool_test() {
    echo "Prepare Ckb-testtool testing environment..."
    pushd crates/tests/test-contract && ../../../target/debug/capsule build && popd
}

run_tests() {
    echo "Running tests..."
    cargo run -p tests $1
}

prepare_capsule_test && \
prepare_testtool_test && \
run_tests $1 && \
echo "Ok"
