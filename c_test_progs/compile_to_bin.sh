#!/usr/bin/bash

# Uses GCC to compile the provided file into a binary file with the same name but a .bin extension.
# The binary is compiled for the RISC-V architecture and is statically linked.

clang -O --target=riscv32-none -march=rv32i -mabi=ilp32 \
    -std=gnu23 -nostdlib -Wall -Wextra \
    -static -fuse-ld=lld $1 -o ${1%.c}.elf
llvm-strip ${1%.c}.elf
llvm-objdump ${1%.c}.elf -d

llvm-objcopy -O binary ${1%.c}.elf ${1%.c}.bin