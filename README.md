# Image Server with Immich Integration

This project contains three binaries:
1. `image-server` - A simple web server that serves images from a directory
2. `immich-fetcher` - A tool to fetch original images from an Immich album
3. `image-transformer` - A tool to transform original images to grayscale PNGs

## Setup

1. Create a `.env` file with your Immich configuration:
```
IMMICH_URL=http://your-immich-server:2283
IMMICH_API_KEY=your_api_key_here
IMMICH_ALBUM_ID=your_album_id_here
```

2. Build the project:
```
cargo build --release
```

## Using the Immich Fetcher

Fetch original images from your Immich album:
```
cargo run --bin immich-fetcher
```

Or with custom parameters:
```
cargo run --bin immich-fetcher -- --immich-url http://your-immich-server:2283 --api-key your_api_key --album-id your_album_id --originals-dir originals --max-images 50
```

## Using the Image Transformer

Transform the downloaded original images to grayscale PNGs:
```
cargo run --bin image-transformer
```

Or with custom parameters:
```
cargo run --bin image-transformer -- --originals-dir originals --output-dir images
```

## Running the Image Server

After fetching and transforming images, run the image server:
```
cargo run --bin image-server
```

Then access the images at: http://localhost:8080/image
