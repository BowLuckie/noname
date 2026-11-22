.PHONY: all run build clean test
.DEFAULT:  run

all: run

run:
	-cargo run -- --emit-tokens --emit-ast --color auto ./examples/main.un 

dump:
	-cargo run -- --emit-tokens --emit-ast --color auto ./examples/main.un > out.txt 2>&1 
	cat out.txt


build:
	cargo build

clean:
	cargo clean

test:
	cargo test
