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
