CURRENT=$(pwd)
cd bootloader
cargo build
cp ./target/x86_64-unknown-uefi/debug/horse-bootloader.efi ../dev-tools
cd $CURRENT
./dev-tools/run_qemu.sh
rm disk.img
