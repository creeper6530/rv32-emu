ENTRY(_start)

SECTIONS
{
    . = 0x10000000;

    .text : AT(0) ALIGN(4)
    {
        KEEP(*(.text.entry))
        KEEP(*(.text._start))
        *(.text .text.*)
    }

    .rodata : AT(LOADADDR(.text) + SIZEOF(.text)) ALIGN(4)
    {
        *(.rodata .rodata.*)
    }

    . = 0x20000000;

    .data : AT(LOADADDR(.rodata) + SIZEOF(.rodata)) ALIGN(4)
    {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    .bss : ALIGN(4)
    {
        *(.bss .bss.*)
        *(.sbss .sbss.* COMMON)
    }

    /DISCARD/ :
    {
        *(.comment)
        *(.eh_frame*)
        *(.note*)
        *(.riscv.attributes)
        *(.debug*)
    }
}