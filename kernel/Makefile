kernel.elf = target/x86_64-unknown-none/release/kernel

build: $(kernel.elf)

$(kernel.elf): src/*
	cargo +nightly build --release

clean:
	cargo clean
