# Build stage
FROM rust:1.81 AS builder

WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

# Image for image-server
FROM debian:bookworm-slim AS image_server
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/image-server /app/
COPY images/ /app/images/

EXPOSE 8080
CMD ["./image-server"]

# Image for immich-fetcher
FROM debian:bookworm-slim AS immich_fetcher
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/immich-fetcher /app/

CMD ["./immich-fetcher"]

# Image for image-transformer
FROM debian:bookworm-slim AS image_transformer
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    imagemagick \
    inotify-tools && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/app/target/release/image-transformer /app/

CMD ["./image-transformer"]
