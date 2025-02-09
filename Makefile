MODE ?= debug
ifeq ($(MODE), release)
	CARGO_MODE = --release
endif

kernel.elf = target/x86_64-unknown-none/$(MODE)/kernel
boot.efi = target/x86_64-unknown-uefi/$(MODE)/boot.efi

build: image

test: image
	qemu-system-x86_64 -m 4G -machine q35 -bios /usr/share/ovmf/OVMF.fd -drive file=image -net none

debug: image
	qemu-system-x86_64 -m 4G -machine q35 -bios /usr/share/ovmf/OVMF.fd -drive file=image -net none -s -S

image: $(kernel.elf) $(boot.efi)
	mkdir -p esp
	sudo mount -o offset=1048576 image esp
	sudo mkdir -p esp/EFI/BOOT
	sudo cp $(kernel.elf) esp/kernel.elf
	sudo cp $(boot.efi) esp/EFI/BOOT/BOOTX64.EFI
	sudo umount esp

$(kernel.elf): kernel/src/*
	cd kernel; cargo build $(CARGO_MODE)

$(boot.efi): $(kernel.elf) boot/src/*
	cd boot; cargo build $(CARGO_MODE)

clean:
	cargo clean
	mkfs.vfat --offset=2048 image
	rm -rf esp
