#!/usr/bin/bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

# Uses GCC to compile the provided file into a binary file with the same name but a .bin extension.
# The binary is compiled for the RISC-V architecture and is statically linked.

# -Wl,-T,"linker.ld": Specifies the linker script to use for the linking process.

mkdir -p target

# Filename expansion patterns that match no files expand to a null string, rather than themselves.
shopt -s nullglob
driver_sources=(drivers/*.c)

clang -O -flto --target=riscv32-none -march=rv32i -mabi=ilp32 -std=gnu23 \
    -ffreestanding -fno-builtin -nostdlib -nostartfiles \
    -Wall -Wextra -fvisibility=hidden \
    -static -fuse-ld=lld \
    -Wl,-T,"linker.ld" \
    "start.S" "$1" "${driver_sources[@]}" -o "target/${1%.c}.elf"
llvm-strip "target/${1%.c}.elf"
llvm-objdump "target/${1%.c}.elf" -d

llvm-objcopy -O binary "target/${1%.c}.elf" "target/${1%.c}.bin"

cd -
