.PHONY: all run build clean
.DEFAULT:  run

all: run

run:
	cargo run

build:
	cargo build

clean:
	cargo clean
