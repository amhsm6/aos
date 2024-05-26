kernel.elf = kernel/target/x86_64-unknown-none/release/kernel
boot.efi = boot/target/x86_64-unknown-uefi/release/boot.efi

build: image

test: image
	qemu-system-x86_64 -bios /usr/share/ovmf/OVMF.fd -drive file=image -net none

dbg: image
	qemu-system-x86_64 -bios /usr/share/ovmf/OVMF.fd -drive file=image -net none -s -S

image: $(kernel.elf) $(boot.efi)
	mkdir -p esp
	sudo mount -o offset=1048576 image esp
	sudo mkdir -p esp/EFI/BOOT
	sudo cp $(kernel.elf) esp/kernel.elf
	sudo cp $(boot.efi) esp/EFI/BOOT/BOOTX64.EFI
	sudo umount esp

$(kernel.elf): kernel/src/*
	make -C kernel build

$(boot.efi): boot/src/*
	make -C boot build

clean:
	mkfs.vfat --offset=2048 image
	rm -rf esp
	make -C kernel clean
	make -C boot clean
