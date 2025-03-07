#!/bin/bash

# This script converts an image to grayscale and applies style transfer
# Usage: ./convert_image.sh <input_path> <output_path>

if [ $# -ne 2 ]; then
    echo "Usage: $0 <input_path> <output_path>"
    exit 1
fi

INPUT_PATH="$1"
OUTPUT_PATH="$2"
TEMP_STYLIZED="/tmp/stylized_$(basename "$OUTPUT_PATH")"
STYLE_IMAGE="${STYLE_IMAGE:-/app/style/style.jpg}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -f "$STYLE_IMAGE" ]; then
    echo "Applying style transfer using style image: $STYLE_IMAGE"
    python3 "$SCRIPT_DIR/stylize.py" "$INPUT_PATH" "$STYLE_IMAGE" "$TEMP_STYLIZED"
else
    echo "Style image not found at $STYLE_IMAGE. Using grayscale image only."
    mv "$INPUT_PATH" "$TEMP_STYLIZED"
fi

convert "$TEMP_STYLIZED" \
    -colorspace Gray \
    -depth 8 \
    -brightness-contrast '0x40' \
    -resize "1072x1448^" \
    -gravity center \
    -crop "1072x1448+0+0" \
    +repage \
    "$OUTPUT_PATH"

exit $?
