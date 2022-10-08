FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:edge
RUN dpkg --add-architecture armhf
RUN apt-get update && apt-get install --assume-yes --upgrade libssl-dev:armhf \
    libgstreamer1.0-dev:armhf  libgstreamer-plugins-base1.0-dev:armhf  \
    gstreamer1.0-plugins-base:armhf  gstreamer1.0-plugins-good:armhf  \
    gstreamer1.0-plugins-bad:armhf  gstreamer1.0-plugins-ugly:armhf  \
    gstreamer1.0-libav:armhf  libgstrtspserver-1.0-dev:armhf  libges-1.0-dev:armhf \
    libglib2.0-dev:armhf
