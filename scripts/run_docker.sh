#!/bin/bash
set -e

# Build the docker image
echo "Building Docker image..."
docker build -t rust-vst-test .

# Run the tests
echo "Running tests in Docker..."
# We mount the current directory to /app
# We use --rm to clean up container after exit
docker run --rm -v "$(pwd):/app" -w /app rust-vst-test \
    bash -c "Xvfb :99 -screen 0 1024x768x24 & sleep 2 && cargo test --all"
