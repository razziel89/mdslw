SHELL := /bin/bash -euo pipefail

SRC := $(shell find src -name "*.rs")
TARGET_DEV := target/debug/mdslw

default: build-dev

build-dev: $(TARGET_DEV)

$(TARGET_DEV): Cargo.lock Cargo.toml $(SRC)
	cargo build -j "$$(nproc --all)"

TARGETS := \
	x86_64-unknown-linux-musl \
	x86_64-apple-darwin \
	aarch64-apple-darwin \
	x86_64-pc-windows-msvc \
	x86_64-pc-windows-gnu

.PHONY: install-toolchains
install-toolchains:
	for target in $(TARGETS); do \
		rustup target add "$${target}" || exit 1; \
	done

# Only perform prod build if dev build works.
build-prod: build-dev
	for target in $(TARGETS); do \
		cargo build --release --target="$${target}"
	done

$(TARGET_DEV): Cargo.lock Cargo.toml $(SRC)
	cargo build -j "$$(nproc --all)"

TEST_MD:= $(sort $(wildcard examples/*_bad.md))

.PHONY: test
test: build-dev
	for input in $(TEST_MD); do \
		output=$${input//_bad./_good.}; \
		diff -q <($(TARGET_DEV) < "$${input}") <(cat "$${output}"); \
	done
