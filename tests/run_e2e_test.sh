#!/bin/bash
set -euo pipefail

ORIGINAL_BLANK_IMAGE="./tests/blank_image.png"
# Create test directories
TEST_ROOT="./test-data"
IMAGE_PATH="$TEST_ROOT/sample-image.png"
MOCK_PORT=35323
ALBUM_ID="test-album"
ASSET_ID="sample-image-asset"
EXIT_CODE=0

rm -rf $TEST_ROOT
# Create all the directories,
# otherwise they will be created in the containers and be owned by root,
# which will make them problematic to clean up
mkdir -p $TEST_ROOT/images
mkdir -p $TEST_ROOT/originals
cp "$ORIGINAL_BLANK_IMAGE" "$IMAGE_PATH"

cargo build --bin mock-immich-server

./target/debug/mock-immich-server \
  --album-id="$ALBUM_ID" \
  --asset-id="$ASSET_ID" \
  --test-image-path="$IMAGE_PATH" \
  --port="$MOCK_PORT" &

MOCK_SERVER_PID=$!

# Wait and test periodically if the server is up
for i in {1..10}; do
  if curl -s -f "http://localhost:$MOCK_PORT/api/assets/$ASSET_ID/original" -o /dev/null; then
    echo "SUCCESS: Mock Server is up"
    break
  else
    echo "Mock Server is not up yet, waiting..."
    sleep 1
  fi
done

# Prebuild containers
docker compose create --build

HOST_IP=$(docker network inspect bridge -f '{{range .IPAM.Config}}{{.Gateway}}{{end}}')

# optionally:
# CONVERSION_SCRIPT=dummy_convert_image.sh

IMMICH_URL="http://$HOST_IP:$MOCK_PORT" \
  IMMICH_API_KEY="dummy-key" \
  IMMICH_ALBUM_ID="$ALBUM_ID" \
  ORIGINALS_DIR=$TEST_ROOT/originals \
  CONVERTED_DIR=$TEST_ROOT/images \
  docker compose up &

COMPOSE_PID=$!

echo "Docker compose PID: $COMPOSE_PID"


# Wait and test periodically if the server is up
for i in {1..10}; do
  if curl -s -f http://localhost:8080/image -o /dev/null; then
    echo "SUCCESS: Server is up"
    break
  else
    echo "Server is not up yet, waiting..."
    sleep 1
  fi
done

FILE="$TEST_ROOT/pic"
echo "Testing image server..."
if curl -s -f http://localhost:8080/image -o $FILE; then
  echo "Image server is serving images"
  if file --mime-type "$FILE" | grep -q "image/png"; then
    echo "SUCCESS: $FILE is a PNG image"
  else
    echo "ERROR: $FILE is not a PNG image"
  fi
else
  echo "ERROR: Image server is not responding or returning an error"
  EXIT_CODE=1
fi

kill $COMPOSE_PID || true

kill -9 $MOCK_SERVER_PID

docker compose down

exit $EXIT_CODE
