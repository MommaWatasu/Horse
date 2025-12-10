/* Linker script for HorseOS kernel - Higher Half Kernel */

/* Virtual address where kernel will be mapped (top 2GB, kernel code model compatible) */
KERNEL_VIRT_BASE = 0xFFFFFFFF80000000;
/* Physical address where kernel is loaded */
KERNEL_PHYS_BASE = 0x100000;

ENTRY(kernel_main)

SECTIONS
{
    /* Start at higher half virtual address */
    . = KERNEL_VIRT_BASE + KERNEL_PHYS_BASE;

    /* Text section */
    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_VIRT_BASE)
    {
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_VIRT_BASE)
    {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_VIRT_BASE)
    {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_VIRT_BASE)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Kernel end marker */
    __kernel_end = .;

    /* Discard unwanted sections */
    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame)
        *(.note.*)
    }
}
