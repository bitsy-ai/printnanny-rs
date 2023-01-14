FROM ubuntu:22.04
ARG DEBIAN_FRONTEND=noninteractive

COPY docker/common.sh docker/lib.sh /
RUN /common.sh

COPY docker/cmake.sh /
RUN /cmake.sh

COPY docker/xargo.sh /
RUN /xargo.sh

RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross

# COPY deny-debian-packages.sh /
# RUN TARGET_ARCH=arm64 /deny-debian-packages.sh \
#     binutils \
#     binutils-aarch64-linux-gnu

COPY docker/qemu.sh /
RUN /qemu.sh aarch64 softmmu

COPY docker/dropbear.sh /
RUN /dropbear.sh

COPY docker/linux-image.sh /
RUN /linux-image.sh aarch64

COPY docker/linux-runner docker/base-runner.sh /

ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_RUNNER="/linux-runner aarch64" \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
    BINDGEN_EXTRA_CLANG_ARGS_aarch64_unknown_linux_gnu="--sysroot=/usr/aarch64-linux-gnu" \
    QEMU_LD_PREFIX=/usr/aarch64-linux-gnu \
    RUST_TEST_THREADS=1 \
    PKG_CONFIG_PATH="/usr/lib/aarch64-linux-gnu/pkgconfig/:${PKG_CONFIG_PATH}"

RUN dpkg --add-architecture arm64
RUN apt-get install -y software-properties-common
RUN apt-add-repository ppa:nnstreamer
RUN apt-get update && apt-get install --assume-yes --upgrade \
    libssl-dev:arm64 \
    libgstreamer1.0-dev:arm64 libgstreamer-plugins-base1.0-dev:arm64 \
    gstreamer1.0-plugins-base:arm64 gstreamer1.0-plugins-good:arm64 \
    gstreamer1.0-plugins-bad:arm64 gstreamer1.0-plugins-ugly:arm64 \
    gstreamer1.0-libav:arm64 libgstrtspserver-1.0-dev:arm64 libges-1.0-dev:arm64 \
    libglib2.0-dev:arm64 \
    nnstreamer:arm64 \
    nnstreamer-tensorflow2-lite:arm64 \
    nnstreamer-dev:arm64 \
    sqlite3:arm64

RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash - && apt-get install -y nodejs build-essential gcc g++ make
