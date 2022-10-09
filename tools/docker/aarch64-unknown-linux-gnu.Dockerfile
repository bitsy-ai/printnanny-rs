FROM ubuntu:22.04
ARG DEBIAN_FRONTEND=noninteractive

COPY common.sh lib.sh /
RUN /common.sh

COPY cmake.sh /
RUN /cmake.sh

COPY xargo.sh /
RUN /xargo.sh

RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross

COPY deny-debian-packages.sh /
RUN TARGET_ARCH=arm64 /deny-debian-packages.sh \
    binutils \
    binutils-aarch64-linux-gnu

COPY qemu.sh /
RUN /qemu.sh aarch64 softmmu

COPY dropbear.sh /
RUN /dropbear.sh

COPY linux-image.sh /
RUN /linux-image.sh aarch64

COPY linux-runner base-runner.sh /

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="/linux-runner aarch64" \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
    BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_gnu="--sysroot=/usr/aarch64-linux-gnu" \
    QEMU_LD_PREFIX=/usr/aarch64-linux-gnu \
    RUST_TEST_THREADS=1 \
    PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig/:${PKG_CONFIG_PATH}"

RUN dpkg --add-architecture arm64
RUN apt-get update && apt-get install --assume-yes --upgrade \
    libssl-dev:arm64 \
    libgstreamer1.0-dev:arm64 libgstreamer-plugins-base1.0-dev:arm64 \
    gstreamer1.0-plugins-base:arm64 gstreamer1.0-plugins-good:arm64 \
    gstreamer1.0-plugins-bad:arm64 gstreamer1.0-plugins-ugly:arm64 \
    gstreamer1.0-libav:arm64 libgstrtspserver-1.0-dev:arm64 libges-1.0-dev:arm64 \
    libglib2.0-dev:arm64
