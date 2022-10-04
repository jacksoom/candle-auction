#!/usr/bin/make -f

all: fmt check schema

fmt:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings

check:
	cargo check --tests

.PHONY: fmt check schema
