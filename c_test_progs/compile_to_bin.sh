#!/usr/bin/bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"

# Uses GCC to compile the provided file into a binary file with the same name but a .bin extension.
# The binary is compiled for the RISC-V architecture and is statically linked.

# -Wl,-T,"$script_dir/linker.ld": Specifies the linker script to use for the linking process.

clang -O -flto --target=riscv32-none -march=rv32i -mabi=ilp32 -std=gnu23 \
    -ffreestanding -fno-builtin -nostdlib -nostartfiles \
    -Wall -Wextra -fvisibility=hidden \
    -static -fuse-ld=lld \
    -Wl,-T,"$script_dir/linker.ld" \
    "$script_dir/start.S" "$1" -o ${1%.c}.elf
llvm-strip ${1%.c}.elf
llvm-objdump ${1%.c}.elf -d

llvm-objcopy -O binary ${1%.c}.elf ${1%.c}.bin
