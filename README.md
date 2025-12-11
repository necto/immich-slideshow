# Image Server with Immich Integration and Style Transfer

This project provides an end-to-end solution for fetching images from Immich, applying style transfer and transformations, and serving them via a web interface. It consists of three main components:

1. `immich-fetcher` - Fetches original images from an Immich album
2. `image-transformer` - Transforms images using neural style transfer and converts them to grayscale
3. `image-server` - A web server that serves the processed images

## Features

- Automatic fetching of images from Immich albums
- Neural style transfer using TensorFlow
- Grayscale conversion and image resizing
- Real-time processing of new images
- Simple web interface to view processed images

## Setup

### Option 1: Using Docker Compose (Recommended)

1. Create a `.env` file with your Immich configuration:
```
IMMICH_URL=http://your-immich-server:2283
IMMICH_API_KEY=your_api_key_here
IMMICH_ALBUM_ID=your_album_id_here
MAX_IMAGES=50
```

2. Add a style image:
```
mkdir -p style
# Copy your desired style image to style/style.jpg
```

3. Create necessary directories:
```
mkdir -p originals images
```

4. Start all services:
```
docker-compose up -d
```

### Option 2: Building and Running Manually

1. Create the same `.env` file as above

2. Build the project:
```
cargo build --release
```

3. Create necessary directories:
```
mkdir -p originals images style
# Copy your desired style image to style/style.jpg
```

## Component Details

### Immich Fetcher

The Immich Fetcher runs as a continuous service that checks for new images every minute:
```
cargo run --bin immich-fetcher
```

Or with custom parameters:
```
cargo run --bin immich-fetcher -- --immich-url http://your-immich-server:2283 --api-key your_api_key --album-id your_album_id --originals-dir originals --max-images 50
```

The service will:
- Download all images from the specified album
- Check for new images every minute
- Skip images that have already been downloaded

### Image Transformer

The Image Transformer applies neural style transfer and converts images to grayscale:
```
cargo run --bin image-transformer
```

Or with custom parameters:
```
cargo run --bin image-transformer -- --originals-dir originals --output-dir images
```

The service will:
- Process all existing images in the originals directory
- Apply neural style transfer using the style image at `style/style.jpg`
- Convert images to grayscale and resize them
- Watch for new files and process them immediately
- Skip images that have already been processed

### Image Server

After fetching and transforming images, run the image server:
```
cargo run --bin image-server
```

Then access the images at: http://localhost:8080/image

#### Image Ordering and Gallery

The image server maintains an explicit ordering of images in `image_order.json` instead of using alphabetical sorting. This allows you to control the sequence in which images are served.

**Viewing the Gallery:**

Access the interactive gallery interface at:
```
http://localhost:8080/all-images
```

This displays all images in their current order with:
- A "Next image to serve" indicator showing which image will be served next
- An interactive grid layout with all images
- File modification dates for each image

**Reordering Images:**

To move an image to a specific position, use the `/all-images` endpoint with query parameters:
```
http://localhost:8080/all-images?image-name=photo.jpg&move-to=0
```

Parameters:
- `image-name`: The filename of the image to move (URL-encoded if necessary)
- `move-to`: The target position (0-based index)

Examples:
```
# Move image5.jpg to the first position
http://localhost:8080/all-images?image-name=image5.jpg&move-to=0

# Move photo.jpg to position 2
http://localhost:8080/all-images?image-name=photo.jpg&move-to=2
```

**Error Handling:**

If you try to reorder an image that doesn't exist in the order list, the server will return a 400 error with a descriptive message:
```
Error: Image 'nonexistent.jpg' not found in order list
```

**Order Persistence:**

The image order is saved to `image_order.json` in the working directory and persists across server restarts. When new images are added to the directory:
- They are inserted right after the current position (the image about to be served next)
- This means a newly added image will appear immediately on the next request, without waiting to cycle through all existing images
- Multiple new images are inserted in the order they appear, all right after the current position

#### Parameter Storage and Control Panel

The image server includes a parameter storage feature that captures HTTP GET parameters and makes them retrievable via a control panel:

**Storing Parameters:**

Pass any GET parameters to the `/image` endpoint:
```
http://localhost:8080/image?param1=18&alfa=x&status=active
```

Parameters are automatically stored in a JSON file with:
- **Value**: The parameter value (URL-decoded)
- **Timestamp**: Unix timestamp (seconds since epoch) when the parameter was last received

**Retrieving Parameters:**

Access the `/control-panel` endpoint to view all stored parameters:
```
http://localhost:8080/control-panel
```

This returns a JSON response like:
```json
{
  "param1": {
    "value": "18",
    "timestamp": 1702000000
  },
  "alfa": {
    "value": "x",
    "timestamp": 1702000000
  },
  "status": {
    "value": "active",
    "timestamp": 1702000000
  }
}
```

**Features:**
- Multiple parameters can be passed in a single request
- Each parameter is stored with its last-received value and timestamp
- New parameter values overwrite previous values for the same parameter
- Other parameters remain unaffected when one parameter is updated
- URL-encoded parameters are automatically decoded (e.g., spaces as %20)
- Parameters persist in `params.json` file in the working directory
- Returns empty JSON object `{}` if no parameters have been stored yet

**Use Cases:**

This feature is particularly useful for tracking status information from kiosk devices:

- **Battery Level Monitoring**: E-ink displays like Kindles can periodically report their battery level by requesting `/image?battery=85&device_id=kindle_1`, allowing you to monitor device health from the control panel
- **Device Status Tracking**: Report WiFi signal strength, memory usage, or other system metrics
- **Uptime Monitoring**: Timestamp tracking allows you to see when devices last reported in

## Style Transfer

The system uses TensorFlow's arbitrary image stylization model to apply artistic styles to your photos. To change the style:

1. Replace the image at `style/style.jpg` with your preferred style image
2. The system will automatically use the new style for future image processing

## Troubleshooting

- If images aren't being fetched, check your Immich API key and album ID
- If style transfer isn't working, ensure the style image exists at the correct path
- Check Docker logs for detailed error messages:
```
docker-compose logs immich-fetcher
docker-compose logs image-transformer
docker-compose logs image-server
```
