FROM ghcr.io/cross-rs/x86_64-unknown-linux-gnu:edge
RUN yum install -y openssl-devel \
    gstreamer-devel gstreamer-universe
