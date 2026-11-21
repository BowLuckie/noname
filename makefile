.PHONY: all run build clean test
.DEFAULT:  run

all: run

run:
	cargo run -- ./examples/main.un

build:
	cargo build

clean:
	cargo clean

test:
	cargo test
