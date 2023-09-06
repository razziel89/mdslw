SHELL := /bin/bash -euo pipefail

SRC := $(shell find src -name "*.rs")
TARGET_DEV := target/debug/mdslw

default: build-dev

build-dev: $(TARGET_DEV)

$(TARGET_DEV): Cargo.lock Cargo.toml $(SRC)
	cargo build -j "$$(nproc --all)"

.PHONY: install-toolchains
install-toolchains:
	rustup target add x86_64-unknown-linux-musl
	rustup target add x86_64-apple-darwin
	# Leave out Apple silicon for now.
	# rustup target add arch64-apple-darwin
	rustup target add x86_64-pc-windows-gnu

# Only perform prod build if dev build works.
build-prod: build-dev
	echo ==== x86_64-unknown-linux-musl ====
	RUSTFLAGS='-C link-arg=-s -C relocation-model=static' \
	cargo build -j "$$(nproc --all)" --release --target="x86_64-unknown-linux-musl"
	echo ==== x86_64-pc-windows-gnu ====
	RUSTFLAGS='-C link-arg=-s' \
	cargo build -j "$$(nproc --all)" --release --target x86_64-pc-windows-gnu

.PHONY: copy-relese-binaries
copy-relese-binaries:
	rm -rf ./dist
	mkdir -p ./dist
	cp target/x86_64-unknown-linux-musl/release/mdslw ./dist/mdslw_x86_64-unknown-linux-musl
	cp target/x86_64-pc-windows-gnu/release/mdslw.exe ./dist/mdslw_x86_64-pc-windows-gnu.exe
	cp target/x86_64-apple-darwin/release/mdslw ./dist/mdslw_x86_64-apple-darwin

TEST_MD:= $(sort $(wildcard examples/*_bad.md))

.PHONY: test
test: build-dev
	for input in $(TEST_MD); do \
		output=$${input//_bad./_good.}; \
		diff -q <($(TARGET_DEV) < "$${input}") <(cat "$${output}"); \
	done
