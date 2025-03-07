#!/bin/bash

# This script converts an image to grayscale using ImageMagick
# Usage: ./convert_image.sh <input_path> <output_path>

if [ $# -ne 2 ]; then
    echo "Usage: $0 <input_path> <output_path>"
    exit 1
fi

INPUT_PATH="$1"
OUTPUT_PATH="$2"

convert "$INPUT_PATH" \
    -colorspace Gray \
    -depth 8 \
    -resize "1072x1448^" \
    -gravity center \
    -crop "1072x1448+0+0" \
    +repage \
    "$OUTPUT_PATH"

exit $?
