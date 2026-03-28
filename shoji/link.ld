/* Linker script for Horse OS user-space programs */

/* Entry point */
ENTRY(_start)

/* User program base address */
/* This should match what the kernel's ELF loader expects */
USER_BASE = 0x400000;

SECTIONS
{
    /* Start at the user base address */
    . = USER_BASE;

    /* Code section */
    .text : ALIGN(4K)
    {
        *(.text .text.*)
    }

    /* Read-only data */
    .rodata : ALIGN(4K)
    {
        *(.rodata .rodata.*)
    }

    /* Initialized data */
    .data : ALIGN(4K)
    {
        *(.data .data.*)
    }

    /* Uninitialized data (BSS) */
    .bss : ALIGN(4K)
    {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }

    /* Discard unnecessary sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.note.*)
        *(.comment)
    }
}
