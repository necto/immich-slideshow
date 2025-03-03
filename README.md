# Image Server with Immich Integration

This project contains two binaries:
1. `image-server` - A simple web server that serves images from a directory
2. `immich-fetcher` - A tool to fetch images from an Immich album

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

Fetch images from your Immich album:
```
cargo run --bin immich-fetcher
```

Or with custom parameters:
```
cargo run --bin immich-fetcher -- --immich-url http://your-immich-server:2283 --api-key your_api_key --album-id your_album_id --output-dir images --max-images 50
```

## Running the Image Server

After fetching images, run the image server:
```
cargo run --bin image-server
```

Then access the images at: http://localhost:8080/image
