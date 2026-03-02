# pg_viewctl

Experimental PostgreSQL extension for view dependency management. Built to solve real problems and to learn PostgreSQL internals.

**Status:** work in progress, everything might and most likely will change

## What it does

Tracks column-level dependencies between views and provides tools for safe schema evolution.

TBD proper documentation.

## Build

Built with Rust and [pgrx](https://github.com/pgcentralfoundation/pgrx). Requires Rust toolchain and a local PostgreSQL install.

```bash
cargo install cargo-pgrx
# adjust to local version
cargo pgrx init --pg18=$(which pg_config)
cargo pgrx test
cargo pgrx run
```

