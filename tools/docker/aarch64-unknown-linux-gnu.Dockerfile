FROM rustembedded/cross:aarch64-unknown-linux-gnu
RUN dpkg --add-architecture arm64
RUN apt-get update && apt-get install --assume-yes libssl-dev:arm64
