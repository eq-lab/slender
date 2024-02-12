default: build

all: test

test: build
	rm -f contracts/pool/src/tests/snapshots/*
	cargo test -p common
	cargo test -p deployer
	cargo test -p s-token --features testutils
	cargo test -p pool --features testutils
	cargo test -p pool budget --features budget -- --test-threads=1
	cargo test -p debt-token

budget: build
	rm -f contracts/pool/src/tests/snapshots/*
	cargo test -p pool budget --features budget -- --test-threads=1

build:
	cargo build --target wasm32-unknown-unknown --release 
	@ls -l target/wasm32-unknown-unknown/release/*.wasm

deploy-contracts:
	(cd deploy/artifacts && shopt -s dotglob; rm -rf *)
	./deploy/scripts/deploy.sh $(env)

init-contracts:
	yarn --cwd integration-tests init-$(env) $(customXlm)

create-bindings:
	./deploy/scripts/create-bindings.sh $(env)

integration-test:
	yarn --cwd integration-tests test-$(env)

upgrade-pool-contract:
	./deploy/scripts/upgrade.sh $(env)

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