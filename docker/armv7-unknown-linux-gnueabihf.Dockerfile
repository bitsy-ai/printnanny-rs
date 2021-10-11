FROM rustembedded/cross:armv7-unknown-linux-gnueabihf-0.2.1
ENV OPENSSL_STATIC=1

RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install -y pkg-config libssl-dev