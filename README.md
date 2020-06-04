# Capsule

A development framework for CKB contract, still in  **WIP** phase.

The name "capsule" is from the dragon ball, which hints our goal is to provide an out-of-box solution.

## Installation

### Requirements

* docker - capsule use `docker` to build contracts and run tests.
* ckb-cli (optional) - capsule require `ckb-cli` to enable contracts deployment feature.

Make sure you installed the dependencies, and their binaries exist in the `PATH`.

### Install preview version

``` sh
cargo install capsule --git https://github.com/nervosnetwork/capsule.git --tag v0.0.1-pre.2
```

## Usage

``` sh
USAGE:
capsule [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    check       Check environment and dependencies
    new         Create a new project
    build       Build contracts
    run         Run command in contract build image
    test        Run tests
    deploy      Deploy contracts, edit deployment.toml to custodian deployment recipe.
    debugger    CKB debugger
    help        Prints this message or the help of the given subcommand(s)
```

### Quick start

``` sh
# check environment
capsule check

# create project
capsule new my-demo
cd my-demo
capsule build
capsule test
```

### Project layout

* `capsule.toml`    - Capsule manifest file.
* `deployment.toml` - Deployment configuration.
* `contracts`       - Contracts directory.
* `tests`           - Contracts tests.
* `build`           - Contracts binaries.
* `migrations`      - Deployment histories.

## Documentation

[WIKI homepage](https://github.com/nervosnetwork/capsule/wiki)

## LICENSE

MIT
