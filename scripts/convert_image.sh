#!/bin/bash

# This script converts an image to grayscale and applies style transfer
# Usage: ./convert_image.sh <input_path> <output_path>

if [ $# -ne 2 ]; then
    echo "Usage: $0 <input_path> <output_path>"
    exit 1
fi

INPUT_PATH="$1"
OUTPUT_PATH="$2"
TEMP_GRAYSCALE="/tmp/grayscale_$(basename "$OUTPUT_PATH")"
STYLE_IMAGE="${STYLE_IMAGE:-/app/style/style.jpg}"

# Step 1: Convert to grayscale
convert "$INPUT_PATH" \
    -colorspace Gray \
    -depth 8 \
    -resize "1072x1448^" \
    -gravity center \
    -crop "1072x1448+0+0" \
    +repage \
    "$TEMP_GRAYSCALE"

# Step 2: Apply style transfer if style image exists
if [ -f "$STYLE_IMAGE" ]; then
    echo "Applying style transfer using style image: $STYLE_IMAGE"
    python3 /app/stylize.py "$TEMP_GRAYSCALE" "$STYLE_IMAGE" "$OUTPUT_PATH"
    rm "$TEMP_GRAYSCALE"
else
    echo "Style image not found at $STYLE_IMAGE. Using grayscale image only."
    mv "$TEMP_GRAYSCALE" "$OUTPUT_PATH"
fi

exit $?
