.PHONY: clean all

all: format lint build test

build:
	cargo build

release:
	cargo build --release

format:
	cargo fmt --all

lint:
	cargo clippy --fix --allow-dirty --allow-staged

test:
	cargo test --all

clean:
	cargo clean
