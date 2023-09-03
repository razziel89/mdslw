SHELL := /bin/bash -euo pipefail

SRC := $(shell find src -name "*.rs")
TARGET_DEV := target/debug/mdslw

default: build-dev

build-dev: $(TARGET_DEV)

$(TARGET_DEV): Cargo.lock Cargo.toml $(SRC)
	cargo build -j "$$(nproc --all)"

TEST_MD:= $(sort $(wildcard examples/*_bad.md))

.PHONY: test
test: build-dev
	for input in $(TEST_MD); do \
		output=$${input//_bad./_good.}; \
		diff -q <($(TARGET_DEV) < "$${input}") <(cat "$${output}"); \
	done
