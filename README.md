# firec

[![](https://docs.rs/firec/badge.svg)](https://docs.rs/firec/) [![](https://img.shields.io/crates/v/firec)](https://crates.io/crates/firec)

`firec` (pronounced "fyrek") is Rust client library to interact with [Firecracker]. It allows you to
create, manipulate, query and stop VMMs.

## Examples

You can see implementations in the [`examples`](./examples/) directory.

## status

Currently heavily in development and therefore expect a lot of API breakage for a while.

Having said that, we'll be following Cargo's SemVer rules so breaking changes will be released in
new minor releases. However, we will only support the latest release.

[Firecracker]: https://github.com/firecracker-microvm/firecracker/
