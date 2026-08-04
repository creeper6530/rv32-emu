#!/usr/bin/bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$script_dir"

for file in *.c; do
    echo "Compiling $file..."
    ./compile.sh "$file"
done

cd ${OLDPWD} # Return to the previous working directory (set by the cd command automatically)
