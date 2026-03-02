EXTENSION = pg_viewctl

.PHONY: build install test run clean

build:
	cargo pgrx package

install:
	cargo pgrx install

test:
	cargo pgrx test

run:
	cargo pgrx run

clean:
	cargo clean
