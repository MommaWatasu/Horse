# Horse
Horse is the OS written in Rust.

## Environment Building
Building is only assumed to be done on Linux.
When you need to build and run this OS, you need these tools:
- Rust nightly
- NASM
- QEMU

To install them, run the following command
```
$ rustup install nightly
$ rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
$ cd /path/to/this/repository
$ rustup override set nightly
$ sudo apt install nasm
$ sudo apt install lld
$ sudo apt install qemu-utils
$ sudo apt install qemu-system-x86
```

## Build OS
Now, you are ready to build HorseOS. Let's build and run with following command:
```
$ cd /path/to/this/repository
$ ./build.sh
```
if you want to get the iso image, add `--save-image` option for `./build.sh`.

You can also build with `cargo make`. It can be installed by:
```
$ cargo install cargo-make
```
Then, build OS:
```
$ cd /path/to/this/repository
$ cargo make run
```
When you only want iso image:
```
$ cargo make make-image
```
