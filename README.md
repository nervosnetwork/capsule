# Capsule

![Github Actions][GH-action-badge] [![Rust crate][rust-crate-badge]](https://crates.io/crates/ckb-capsule)

[GH-action-badge]: https://img.shields.io/github/actions/workflow/status/nervosnetwork/capsule/rust.yml?branch=develop&style=flat
[rust-crate-badge]: https://img.shields.io/crates/v/ckb-capsule?style=flat

> ⚠️ **DEPRECATION NOTICE**: This project is no longer maintained. Please use [ckb-script-templates](https://github.com/cryptape/ckb-script-templates).

**Capsule is an out-of-box development framework for creating smart contract on Nervos' CKB.**

Capsule consists of:

- Capsule CLI - Scaffolding tool.

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

- cargo and rust - Capsule uses `cargo` to generate Rust contracts and run tests. https://www.rust-lang.org/tools/install.
- docker - Capsule uses `docker` container to reproducible build contracts. It's also used by cross. https://docs.docker.com/get-docker/
- cross. Capsule uses `cross` to build rust contracts. Install with

```command
$ cargo install cross --git https://github.com/cross-rs/cross --rev=6982b6c
```

Note: All commands must be accessible in the `PATH` in order for them to be used by Capsule.

Note: The current user must have permission to manage Docker instances. [How to manage Docker as a non-root user.](https://docs.docker.com/engine/install/linux-postinstall/)

### Install binary

[Download the latest release](https://github.com/nervosnetwork/capsule/releases/latest)

### Cargo install

Install the latest version

``` sh
cargo install ckb-capsule --locked
```

Install the develop branch

``` sh
cargo install ckb-capsule --git https://github.com/nervosnetwork/capsule.git --branch develop --locked
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
* `contracts`       - Contracts directory.
* `tests`           - Contracts tests.
* `build`           - Contracts binaries.

## Documentation

[Capsule Wiki on GitHub](https://github.com/nervosnetwork/capsule/wiki)

### Upgrading to Capsule 0.10

[Upgrade an existing project to capsule 0.10](https://github.com/nervosnetwork/capsule/wiki/Upgrade-an-existing-project-to-capsule-0.10)

## LICENSE

[MIT](https://github.com/nervosnetwork/capsule/blob/master/LICENSE)
