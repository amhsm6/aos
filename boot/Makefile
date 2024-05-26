boot.efi = target/x86_64-unknown-uefi/release/boot.efi

build: $(boot.efi)

$(boot.efi): src/*
	cargo build --target=x86_64-unknown-uefi --release

clean:
	cargo clean
