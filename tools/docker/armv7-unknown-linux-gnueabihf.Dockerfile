FROM ubuntu:22.04
ARG DEBIAN_FRONTEND=noninteractive

COPY docker/common.sh docker/lib.sh /
RUN /common.sh

COPY docker/cmake.sh /
RUN /cmake.sh

COPY docker/xargo.sh /
RUN /xargo.sh

RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    g++-arm-linux-gnueabihf \
    libc6-dev-armhf-cross

# COPY deny-debian-packages.sh /
# RUN TARGET_ARCH=armhf /deny-debian-packages.sh \
#     binutils \
#     binutils-arm-linux-gnueabihf

COPY docker/qemu.sh /
RUN /qemu.sh arm softmmu

COPY docker/dropbear.sh /
RUN /dropbear.sh

COPY docker/linux-image.sh /
RUN /linux-image.sh armv7

COPY docker/linux-runner docker/base-runner.sh /

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc \
    CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_RUNNER="/linux-runner armv7hf" \
    CC_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc \
    CXX_armv7_unknown_linux_gnueabihf=arm-linux-gnueabihf-g++ \
    BINDGEN_EXTRA_CLANG_ARGS_armv7_unknown_linux_gnueabihf="--sysroot=/usr/arm-linux-gnueabihf" \
    QEMU_LD_PREFIX=/usr/arm-linux-gnueabihf \
    RUST_TEST_THREADS=1 \
    PKG_CONFIG_PATH="/usr/lib/arm-linux-gnueabihf/pkgconfig/:${PKG_CONFIG_PATH}"

RUN dpkg --add-architecture armhf
RUN apt-get update && apt-get install --assume-yes --upgrade libssl-dev:armhf \
    libgstreamer1.0-dev:armhf  libgstreamer-plugins-base1.0-dev:armhf  \
    gstreamer1.0-plugins-base:armhf  gstreamer1.0-plugins-good:armhf  \
    gstreamer1.0-plugins-bad:armhf  gstreamer1.0-plugins-ugly:armhf  \
    gstreamer1.0-libav:armhf  libgstrtspserver-1.0-dev:armhf  libges-1.0-dev:armhf \
    libglib2.0-dev:armhf
