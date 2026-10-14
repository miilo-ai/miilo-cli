.PHONY: test test-rust test-rust-local test-node test-node-package smoke build

CARGO_HOME ?= /tmp/miilo-cli-cargo
NPM_CONFIG_CACHE ?= /tmp/miilo-cli-npm

test: test-rust-local test-node

test-rust:
	CARGO_HOME="$(CARGO_HOME)" cargo test --workspace --locked

test-rust-local:
	CARGO_HOME="$(CARGO_HOME)" ./scripts/test-rust-local.sh

test-node:
	cd packages/miilo-cli-node && NPM_CONFIG_CACHE="$(NPM_CONFIG_CACHE)" ../../scripts/node-modern.sh npm ci --ignore-scripts
	cd packages/miilo-cli-node && NPM_CONFIG_CACHE="$(NPM_CONFIG_CACHE)" ../../scripts/node-modern.sh npm test
	NPM_CONFIG_CACHE="$(NPM_CONFIG_CACHE)" ./scripts/test-node-package.sh

test-node-package:
	NPM_CONFIG_CACHE="$(NPM_CONFIG_CACHE)" ./scripts/test-node-package.sh

smoke:
	./scripts/smoke.sh

build:
	CARGO_HOME="$(CARGO_HOME)" cargo build --release -p miilo-cli
	cd packages/miilo-cli-node && NPM_CONFIG_CACHE="$(NPM_CONFIG_CACHE)" ../../scripts/node-modern.sh npm run build
