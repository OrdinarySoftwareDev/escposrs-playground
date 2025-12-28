#!/bin/bash
# ESC/POS Character Map Printer (Latin-2)
# Outputs all characters 32–255 for a given codepage to stdout
# Usage: ./char-map.sh <codepage_number>

if [ $# -lt 1 ]; then
    echo "Usage: $0 <codepage_number>" >&2
    exit 1
fi

CODEPAGE=$1

# Reset printer
printf '\033@'

# Select codepage
# ESC t n
printf '\033t%b' "$(printf '\\%03o' "$CODEPAGE")"

# Print ASCII + extended characters
for byte in $(seq 32 255); do
    printf '%b' "\\$(printf '%03o' "$byte")"
done

# Optional: cut paper at the end
printf '\035V\101\003'

# Add a newline for readability in stdout
printf '\n'
