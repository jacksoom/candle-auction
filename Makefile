#!/usr/bin/make -f

all: fmt check schema test

fmt:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings

check:
	cargo check --tests

test:
	cargo test

schema:
	cargo run --package candle_auction --example gen
	
.PHONY: fmt check schema test
