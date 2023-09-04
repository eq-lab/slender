default: build

all: test

test: build
	rm -f pool/src/tests/snapshots/*
	cargo test -p common
	cargo test -p deployer
	cargo test -p s-token --features testutils
	cargo test -p pool --features testutils
	cargo test -p pool budget --features budget -- --test-threads=1
	cargo test -p debt-token

budget: build
	cargo test -p pool budget --features budget -- --test-threads=1 --nocapture

integration-test: build
	yarn --cwd integration-tests test-$(env)

build:
	cargo build --target wasm32-unknown-unknown --release 
	@ls -l target/wasm32-unknown-unknown/release/*.wasm

build-exceeded-limit-fix:
	cargo build --target wasm32-unknown-unknown --release --features exceeded-limit-fix

check:
	cargo check --target wasm32-unknown-unknown --release

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

clippy:
	cargo clippy

clean:
	cargo clean
