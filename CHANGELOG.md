# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com).

## [v0.10.2] - 2023-11-3

Misc:

* chore: upgrade to ckb v0.111

## [v0.10.1] - 2023-06-19

Bug fixes:

* fix: fix address parsing #125
* feat: remap path prefixs on release #122

Misc:

* Upgrade ckb-std and ckb-testtool dependencies


CKB Testtool:

* feat(ckb-testtool): Support Type ID #123


Full Changelog: https://github.com/nervosnetwork/capsule/compare/v0.10.0...v0.10.1

## [v0.10.0] - 2023-05-08

Capsule now uses `cross` to manage Rust contracts building. You can learn more details about this change in issue [#106](https://github.com/nervosnetwork/capsule/pull/106), and the wiki page [Upgrade an existing project to capsule 0.10](https://github.com/nervosnetwork/capsule/wiki/Upgrade-an-existing-project-to-capsule-0.10) provides instructions for upgrading an existing project to capsule `v0.10.0`.

Feature:

* Use `cross` to build Rust contracts #106
* feat: update ckb-std and use stable rust #118

Misc:

* docs: add link to upgrade an existing project #112
* Fix check command #113

Full Changelog: https://github.com/nervosnetwork/capsule/compare/v0.9.2...v0.10.0

## [v0.9.2] - 2023-04-24

Misc:

* chore: generate project and contract without using docker #91
* chore: run 'capsule test' without docker #96
* chore: update README #98
* chore(ci): build binaries on ubuntu-20.04 #102

Full Changelog: https://github.com/nervosnetwork/capsule/compare/v0.9.1...v0.9.2

## [v0.9.1] - 2023-04-18

Features:

* Experimental lua template support #78 #81

Misc:

* Improve version compatibility determination #82
* chore: update deps #86
* chore: remove dependency simple-jsonrpc-client and update reqwest #83
* chore: add ckb-testtool to capsule repository #90

Full Changelog: https://github.com/nervosnetwork/capsule/compare/v0.9.0...v0.9.1
