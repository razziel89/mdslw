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

.PHONY: test
test:
	cargo test


# Extract languages requested by the code to keep them in sync.
LANGS := $(shell grep -o '/// Supported languages are:\( *[a-z][a-z]\)* *' ./src/main.rs | awk -F: '{print $$2}' | tr -s '[:space:]')
LANG_SUPPRESSION_URL := https://raw.githubusercontent.com/unicode-org/cldr-json/main/cldr-json/cldr-segments-full/segments
LANG_SUPPRESSION_JQ := .segments.segmentations.SentenceBreak.standard[].suppression

# Retrieve the list of keep words according to unicode. Also make sure each file
# ends on an empty line to avoid problems when processing them later.
.PHONY: build-language-files
build-language-files:
	mkdir -p ./src/lang/
	for lang in $(LANGS); do \
		echo >&2 "building: $${lang}" && \
		curl -sSf "$(LANG_SUPPRESSION_URL)/$${lang}/suppressions.json" \
		| jq -r "$(LANG_SUPPRESSION_JQ)" > "./src/lang/$${lang}" \
		|| exit 1 && \
		echo >> "./src/lang/$${lang}"; \
	done

COVERAGE := .coverage.html
PROFRAW := .coverage.profraw
PROFDATA := .coverage.profdata
RUSTC_ROOT := $(shell rustc --print sysroot)
PROF_BIN := $(shell find $(RUSTC_ROOT) -name "llvm-profdata" | head -n1)
COV_BIN := $(shell find $(RUSTC_ROOT) -name "llvm-cov" | head -n1)

.PHONY: coverage
coverage:
	rm -f "$(COVERAGE)" "$(PROFRAW)" "$(PROFDATA)"
	# Install dependencies
	rustup component add llvm-tools
	cargo install rustfilt
	# Build stand-alone test executable.
	RUSTFLAGS="-C instrument-coverage=all" \
		cargo build --tests
	# Find and run executable to generate coverage report.
	exe=$$( \
		find target/debug/deps/ -executable -name "mdslw-*" \
		| xargs ls -t | head -n1 \
	) && \
	LLVM_PROFILE_FILE="$(PROFRAW)" "$${exe}" && \
	"$(PROF_BIN)" merge -sparse "$(PROFRAW)" -o "$(PROFDATA)" && \
	"$(COV_BIN)" show -Xdemangler=rustfilt "$${exe}" \
		--format=html \
  	--instr-profile="$(PROFDATA)" \
  	--show-line-counts-or-regions \
  	--show-instantiations \
		--sources "$$(readlink -e src)" \
		> "$(COVERAGE)"
	# Show it.
	xdg-open "$(COVERAGE)"
