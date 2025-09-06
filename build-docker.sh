#!/bin/bash

# Docker-based cross-compilation script
# Usage: ./build-docker.sh [target-arch]
# Example: ./build-docker.sh aarch64

set -e

TARGET_ARCH="${1:-aarch64}"

case $TARGET_ARCH in
    "aarch64")
        RUST_TARGET="aarch64-unknown-linux-musl"
        DOCKER_PLATFORM="linux/arm64"
        echo "🎯 Building for ARM64 (Raspberry Pi 4+)"
        ;;
    "armv7")
        RUST_TARGET="armv7-unknown-linux-musleabihf"
        DOCKER_PLATFORM="linux/arm/v7"
        echo "🎯 Building for ARMv7 (Raspberry Pi 3 and older)"
        ;;
    *)
        echo "❌ Unsupported architecture: $TARGET_ARCH"
        echo "Supported: aarch64, armv7"
        exit 1
        ;;
esac

echo "🐳 === DOCKER CROSS-COMPILATION ==="
echo "Target Arch: $TARGET_ARCH ($RUST_TARGET)"
echo "Docker Platform: $DOCKER_PLATFORM"
echo ""

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo "❌ Docker not found!"
    echo "Please install Docker Desktop for Mac"
    exit 1
fi

# Create Dockerfile for cross-compilation
cat > Dockerfile.build << EOF
FROM --platform=$DOCKER_PLATFORM rust:1.75-alpine

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN rustup target add $RUST_TARGET
RUN cargo build --target $RUST_TARGET --release

CMD ["sh"]
EOF

echo "🔨 Building with Docker..."

# Build image and extract binary
docker build -f Dockerfile.build -t printer-proxy-builder:$TARGET_ARCH .

# Create container and copy binary
CONTAINER_ID=$(docker create printer-proxy-builder:$TARGET_ARCH)
docker cp "$CONTAINER_ID:/app/target/$RUST_TARGET/release/printer-proxy" "./printer-proxy-$TARGET_ARCH"
docker rm "$CONTAINER_ID"

# Cleanup
rm Dockerfile.build

echo "✅ Cross-compilation complete!"
echo "📦 Binary: printer-proxy-$TARGET_ARCH"
echo "📊 Size: $(du -h printer-proxy-$TARGET_ARCH | cut -f1)"

# Make executable
chmod +x "printer-proxy-$TARGET_ARCH"

echo ""
echo "🚀 To deploy:"
echo "./deploy.sh pi@your-pi.local $TARGET_ARCH"
