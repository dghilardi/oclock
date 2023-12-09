# Oclock
[![Test Status](https://github.com/dghilardi/oclock/workflows/Tests/badge.svg?event=push)](https://github.com/dghilardi/oclock/actions)
[![Crate](https://img.shields.io/crates/v/oclock.svg)](https://crates.io/crates/oclock)

Time tracking utility software

## Quick start

Install using cargo:

```shell
cargo install oclock
```

Launch oclock daemon (in server mode)

```shell
oclock server
```

Interact with the daemon using the client mode

```shell
oclock client list-tasks
```

Available commands can be listed with

```shell
oclock client --help
```