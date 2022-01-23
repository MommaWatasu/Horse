# Horse
Horse is the OS written in Rust.

## Build
When you need to build and run this OS, you need these tools:
- Rust nightly
- QEMU

To install them, run the following command
```
#If you haven't installed Rust, you need to install it too.
rustup install nightly
sudo apt install lld
sudo apt install qemu
sudo apt install qemu-utils
sudo apt install qemu-system-x86
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
```

after this, all you have to do in order to build this OS is run build.sh under the repository.
