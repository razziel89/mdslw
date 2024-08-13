SHELL := /bin/bash -euo pipefail

SRC := $(shell find src -name "*.rs")
TARGET_DEV := target/debug/mdslw
TARGET_PROD := target/x86_64-unknown-linux-musl/release/mdslw

default: build-dev

build-dev: $(TARGET_DEV)

$(TARGET_DEV): Cargo.lock Cargo.toml $(SRC)
	cargo build

.PHONY: install-toolchains
install-toolchains:
	rustup target add x86_64-unknown-linux-musl
	rustup target add armv7-unknown-linux-gnueabihf
	rustup target add x86_64-pc-windows-gnu

build-prod: $(TARGET_PROD)

# Build prod for the dev system.
$(TARGET_PROD): Cargo.lock Cargo.toml $(SRC)
	RUSTFLAGS='-Dwarnings -C link-arg=-s -C relocation-model=static' \
	cargo build -j "$$(nproc --all)" --release --target=x86_64-unknown-linux-musl

.PHONY: build-prod-all
build-prod-all:
	echo ==== x86_64-unknown-linux-musl ====
	$(MAKE) --always-make build-prod
	echo ==== armv7-unknown-linux-gnueabihf ====
	RUSTFLAGS='-Dwarnings -C link-arg=-s' \
	cargo build -j "$$(nproc --all)" --release --target=armv7-unknown-linux-gnueabihf
	echo ==== x86_64-pc-windows-gnu ====
	RUSTFLAGS='-Dwarnings -C link-arg=-s' \
	cargo build -j "$$(nproc --all)" --release --target x86_64-pc-windows-gnu

.PHONY: copy-relese-binaries
copy-relese-binaries:
	rm -rf ./dist
	mkdir -p ./dist
	cp target/x86_64-unknown-linux-musl/release/mdslw ./dist/mdslw_x86_64-unknown-linux-musl
	cp target/armv7-unknown-linux-gnueabihf/release/mdslw ./dist/mdslw_armv7-unknown-linux-gnueabihf
	cp target/x86_64-pc-windows-gnu/release/mdslw.exe ./dist/mdslw_x86_64-pc-windows-gnu.exe

.PHONY: test
test:
	RUSTFLAGS="-Dwarnings" cargo test
	$(MAKE) test-features test-langs assert-version-tag

FEATURES := $(shell grep "/// {n}   \* [a-z-]* => " src/cfg.rs | awk '{print $$4}' | tr '\n' ',' | sed 's/,$$//')

.PHONY: test-features
test-features:
	[[ -n "$(FEATURES)" ]]
	RUSTFLAGS="-Dwarnings" cargo run -- --features="$(FEATURES)" <<< "markdown"

.PHONY: assert-version-tag
assert-version-tag:
	# Extract tag and compare it to the version known by mdslw. When not run on a
	# tag, this target checks that the version known by the tool is not identical
	# to any existing tag. When run on a tag, it checks that the version known is
	# identical to the current tag.
	echo >&2 "Tags: $$(git tag --list | tr '\n' ' ')"
	version=$$(RUSTFLAGS="-Dwarnings" cargo run -- --version | awk '{print $$2'}) && \
	echo >&2 "Version: $${version}" && \
	tag=$$(git describe --exact-match --tags | sed 's/^v//' || :) && \
	if [[ -n "$${tag}" ]]; then \
		if [[ "$${tag}" != "$${version}" ]]; then \
			echo >&2 "Version tag $${tag} does not match tool version $${version}."; \
			exit 1; \
		fi; \
	else \
		tags=$$(git tag --list) && match= && \
		for t in $${tags}; do \
			if [[ "$${version}" == "$$t" ]]; then match="$$t"; fi; \
		done && \
		if [[ -n "$${match-}" ]]; then \
			echo >&2 "Found an existing matching git version tag: $$match"; \
			exit 1; \
		fi; \
	fi

.PHONY: lint
lint:
	rustup component add clippy
	RUSTFLAGS="-Dwarnings" cargo check --all-features --all-targets
	RUSTFLAGS="-Dwarnings" cargo clippy --all-features --all-targets --no-deps

# Extract languages requested by the code to keep them in sync.
LANGS := $(shell grep -o '/// Supported languages are:\( *[a-z][a-z]\)* *' ./src/cfg.rs | awk -F: '{print $$2}' | tr -s '[:space:]')
LANG_SUPPRESSION_URL := https://raw.githubusercontent.com/unicode-org/cldr-json/main/cldr-json/cldr-segments-full/segments
LANG_SUPPRESSION_JQ := .segments.segmentations.SentenceBreak.standard[].suppression

# Retrieve the list of keep words according to unicode. Also make sure each file
# ends on an empty line to avoid problems when processing them later.
.PHONY: build-language-files
build-language-files:
	[[ -n "$(LANGS)" ]]
	mkdir -p ./src/lang/
	for lang in $(LANGS); do \
		echo >&2 "building: $${lang}" && \
		curl -sSf "$(LANG_SUPPRESSION_URL)/$${lang}/suppressions.json" \
		| jq -r "$(LANG_SUPPRESSION_JQ)" > "./src/lang/$${lang}" \
		|| exit 1 && \
		echo >> "./src/lang/$${lang}"; \
	done

.PHONY: test-langs
test-langs:
	[[ -n "$(LANGS)" ]]
	RUSTFLAGS="-Dwarnings" cargo run -- --lang="$(LANGS) ac" <<< "markdown"

COVERAGE := .coverage.html
PROFRAW := .coverage.profraw
PROFDATA := .coverage.profdata
RUSTC_ROOT := $(shell rustc --print sysroot)
LLVM_PROFILE_FILE := $(PROFRAW)
export LLVM_PROFILE_FILE
MIN_COV_PERCENT := 80

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
	prof_exe=$$(find $(RUSTC_ROOT) -executable -name "llvm-profdata" | head -n1) && \
	cov_exe=$$(find $(RUSTC_ROOT) -executable -name "llvm-cov" | head -n1) && \
	"$${exe}" && \
	"$${prof_exe}" merge \
		-sparse "$(PROFRAW)" -o "$(PROFDATA)" && \
	"$${cov_exe}" show \
		-Xdemangler=rustfilt "$${exe}" \
		--format=html \
		--instr-profile="$(PROFDATA)" \
		--show-line-counts-or-regions \
		--show-instantiations \
		--show-branches=count \
		--sources "$$(readlink -e src)" \
		> "$(COVERAGE)" && \
	if [[ -t 1 ]]; then xdg-open "$(COVERAGE)"; fi && \
	"$${cov_exe}" export \
		-Xdemangler=rustfilt "$${exe}" \
		--format=text \
		--instr-profile="$(PROFDATA)" \
		--sources "$$(readlink -e src)" \
		| jq -r ".data[].totals.lines.percent" \
		| awk '{if ($$1<$(MIN_COV_PERCENT)) \
			{printf("coverage low: %.2f%%<$(MIN_COV_PERCENT)%%\n", $$1); exit(1)} \
			else{printf("coverage OK: %.2f%%\n", $$1)} \
		}' >&2
