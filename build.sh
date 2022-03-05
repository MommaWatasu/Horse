cd bootloader
cargo build
cp ./target/x86_64-unknown-uefi/debug/horse-bootloader.efi ../dev-tools
cd ../kernel
cargo build
cp ./horse-kernel ../dev-tools
cd ../
./dev-tools/run_qemu.sh
if [[ $1 != "--save-img" ]]; then
    rm disk.iso
fi
