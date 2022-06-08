FROM ghcr.io/cross-rs/armv7-unknown-linux-gnueabihf:edge
RUN dpkg --add-architecture armhf
RUN apt-get update && apt-get install --assume-yes libssl-dev:armhf
