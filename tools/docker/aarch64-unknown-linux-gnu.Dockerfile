FROM ghcr.io/cross-rs/aarch64-unknown-linux-gnu:edge
RUN dpkg --add-architecture arm64
RUN apt-get update && apt-get install --assume-yes \
    libssl-dev:arm64 \
    libgstreamer1.0-dev:arm64 libgstreamer-plugins-base1.0-dev:arm64 \
    gstreamer1.0-plugins-base:arm64 gstreamer1.0-plugins-good:arm64 \
    gstreamer1.0-plugins-bad:arm64 gstreamer1.0-plugins-ugly:arm64 \
    gstreamer1.0-libav:arm64 libgstrtspserver-1.0-dev:arm64 libges-1.0-dev:arm64

