#!/bin/sh -ex

EFI_FILE=$1
ANOTHER_FILE=$2
DISK_IMG=./Horse.iso
MOUNT_POINT=./mnt
EFI_FILE=horse-bootloader.efi
ANOTHER_FILE=./kernel/horse-kernel

./dev-tools/make_image.sh $DISK_IMG $MOUNT_POINT $EFI_FILE $ANOTHER_FILE
./dev-tools/run_image.sh $DISK_IMG
