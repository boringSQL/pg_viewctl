# pg_viewctl

Experimental PostgreSQL extension for view dependency management. Built to solve real problems and to learn PostgreSQL internals.

**Status:** work in progress, everything might and most likely will change

## What it does

Tracks column-level dependencies between views and provides tools for safe schema evolution.

TBD proper documentation.

## Build

Requires PostgreSQL dev headers.

```bash
make && make install && make installcheck
```

