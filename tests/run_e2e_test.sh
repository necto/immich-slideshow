#!/bin/bash
set -e

# Create test directories
TEST_ROOT="./test-data"
mkdir -p "$TEST_ROOT/originals"
mkdir -p "$TEST_ROOT/images"
mkdir -p "$TEST_ROOT/style"

# Create test image
cargo run --bin create-test-image

# Copy test image to style directory
cp tests/test_image.jpg "$TEST_ROOT/style/style.jpg"

# Make sure the conversion script is executable
chmod +x conversion/dummy_convert_image.sh

# Start the Docker test environment
docker-compose -f docker-compose.test.yml up --build -d

echo "Waiting for services to start..."
sleep 5

# Check if files were downloaded to originals
echo "Checking if images were downloaded..."
ORIGINALS_COUNT=$(ls -1 "$TEST_ROOT/originals" | wc -l)
if [ "$ORIGINALS_COUNT" -eq 0 ]; then
  echo "ERROR: No images were downloaded to the originals directory"
  docker-compose -f docker-compose.test.yml logs immich-fetcher
  docker-compose -f docker-compose.test.yml down
  exit 1
fi

echo "Found $ORIGINALS_COUNT files in originals directory"

# Check if files were transformed
echo "Checking if images were transformed..."
IMAGES_COUNT=$(ls -1 "$TEST_ROOT/images" | wc -l)
if [ "$IMAGES_COUNT" -eq 0 ]; then
  echo "ERROR: No images were transformed in the images directory"
  docker-compose -f docker-compose.test.yml logs image-transformer
  docker-compose -f docker-compose.test.yml down
  exit 1
fi

echo "Found $IMAGES_COUNT files in images directory"

# Test if the image server is responding
echo "Testing image server..."
if curl -s -f http://localhost:8081/image > /dev/null; then
  echo "SUCCESS: Image server is serving images"
else
  echo "ERROR: Image server is not responding or returning an error"
  docker-compose -f docker-compose.test.yml logs image-server
  docker-compose -f docker-compose.test.yml down
  exit 1
fi

# Clean up
docker-compose -f docker-compose.test.yml down
echo "Test completed successfully!"
