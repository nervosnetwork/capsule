# Capsule

![Github Actions][GH-action-badge] [![Rust crate][rust-crate-badge]](https://crates.io/crates/ckb-capsule)

[GH-action-badge]: https://img.shields.io/github/actions/workflow/status/nervosnetwork/capsule/rust.yml?branch=develop&style=flat
[rust-crate-badge]: https://img.shields.io/crates/v/ckb-capsule?style=flat


**Capsule is an out-of-box development framework for creating smart contract on Nervos' CKB.**

Capsule consists of:

- Capsule CLI - Scaffolding tool.
- [CKB-testtool](https://github.com/nervosnetwork/capsule/tree/develop/crates/testtool) - CKB scripts testing framework.

CKB supports several programming languages for writing scripts, and the language supporting libraries are maintained in the following repositories:

- [ckb-std](https://github.com/nervosnetwork/ckb-std) - Rust
- [ckb-c-stdlib](https://github.com/nervosnetwork/ckb-c-stdlib) - C
- [ckb-lua](https://github.com/nervosnetwork/ckb-lua) - Lua


![Capsule](./capsule.jpg)

## Installation

### Supported Environments

- Linux
- macOS
- Windows (WSL2)

### Prerequisites

The following must be installed and available to use Capsule.

- Docker - Capsule uses `docker` to build contracts and run tests. https://docs.docker.com/get-docker/
- ckb-cli (optional) - Capsule requires `ckb-cli` to enable the smart contract deployment feature. https://github.com/nervosnetwork/ckb-cli/releases

Note: Docker and ckb-cli must be accessible in the `PATH` in order for them to be used by Capsule.

Note: The current user must have permission to manage Docker instances. [How to manage Docker as a non-root user.](https://docs.docker.com/engine/install/linux-postinstall/)

### Install binary

[Download the latest release](https://github.com/nervosnetwork/capsule/releases/latest)

### Cargo install

Install the latest version

``` sh
cargo install ckb-capsule
```

Install the develop branch

``` sh
cargo install ckb-capsule --git https://github.com/nervosnetwork/capsule.git --branch develop
```

## Usage

``` sh
capsule help
```

### Quick Start

``` sh
# check environment
capsule check

# create project
capsule new my-demo
cd my-demo
capsule build
capsule test
```

### Project Layout

* `capsule.toml`    - Capsule manifest file.
* `deployment.toml` - Deployment configuration.
* `contracts`       - Contracts directory.
* `tests`           - Contracts tests.
* `build`           - Contracts binaries.
* `migrations`      - Deployment histories.

## Documentation

[Capsule Wiki on GitHub](https://github.com/nervosnetwork/capsule/wiki)

## LICENSE

[MIT](https://github.com/nervosnetwork/capsule/blob/master/LICENSE)
