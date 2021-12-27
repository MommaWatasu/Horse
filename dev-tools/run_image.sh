#!/bin/sh -ex

if [ $# -lt 1 ]
then
    echo "Usage: $0 <image name>"
    exit 1
fi

DISK_IMG=$1

if [ ! -f $DISK_IMG ]
then
    echo "No such file: $DISK_IMG"
    exit 1
fi

qemu-system-x86_64 \
    -m 1G \
    -drive if=pflash,format=raw,readonly,file=./dev-tools/OVMF/OVMF_CODE.fd \
    -drive if=pflash,format=raw,file=./dev-tools/OVMF/OVMF_VARS.fd \
    -drive if=ide,index=0,media=disk,format=raw,file=$DISK_IMG \
    -device nec-usb-xhci,id=xhci \
    -device usb-mouse -device usb-kbd \
    -monitor stdio \
    $QEMU_OPTS
