FROM rust:slim-bookworm

# Install build dependencies, X11, and Vulkan drivers (Lavapipe)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libx11-dev \
    libxcursor-dev \
    libxi-dev \
    libgl1-mesa-dev \
    libvulkan-dev \
    mesa-vulkan-drivers \
    vulkan-tools \
    xvfb \
    git \
    make \
    x11-xserver-utils \
    protobuf-compiler \
    glslang-tools \
    vulkan-validationlayers \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Configure Xvfb
ENV DISPLAY=:99

# Copy entrypoint script if we had one, but we'll use a command in run_docker.sh
