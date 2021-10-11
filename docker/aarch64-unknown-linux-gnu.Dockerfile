FROM rustembedded/cross:aarch64-unknown-linux-gnu-0.2.1
ENV OPENSSL_STATIC=1
ENV PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y pkg-config libssl-dev