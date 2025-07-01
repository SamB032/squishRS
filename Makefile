# Project metadata
VERSION := 1.0.0
TAG := v$(VERSION)
CRATE_NAME := squishrs

.PHONY: all build test clean

all: build

build:
	cargo build --release

fmt:
	cargo fmt

test:
	cargo test

clean:
	cargo clean
