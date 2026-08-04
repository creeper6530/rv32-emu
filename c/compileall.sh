#!/usr/bin/bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

for file in *.c; do
    ./compile.sh "$file"
done

cd -
