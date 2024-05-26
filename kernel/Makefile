kernel.elf = target/x86_64-unknown-none/release/kernel

build: $(kernel.elf)

$(kernel.elf): src/*
	cargo +nightly build --target=x86_64-unknown-none --release

clean:
	cargo clean
