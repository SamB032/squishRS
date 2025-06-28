# Project metadata
VERSION := 1.0.0
TAG := v$(VERSION)
CRATE_NAME := squishrs

.PHONY: all build test release tag push publish clean

all: build

build:
	cargo build --release

test:
	cargo test

tag:
	git tag -a $(TAG) -m "Release $(TAG)"

push:
	git push origin main
	git push origin $(TAG)

publish:
	cargo publish

clean:
	cargo clean

release: build test tag push
