FROM ubuntu:22.04

ARG DEBIAN_FRONTEND=noninteractive

COPY common.sh lib.sh /
RUN /common.sh

COPY cmake.sh /
RUN /cmake.sh

COPY xargo.sh /
RUN /xargo.sh

COPY qemu.sh /
RUN /qemu.sh x86_64 softmmu

COPY dropbear.sh /
RUN /dropbear.sh

COPY linux-image.sh /
RUN /linux-image.sh x86_64

COPY linux-runner /

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="/linux-runner x86_64"

RUN apt-get update -y && apt-get install -y --upgrade \
    libssl-dev \
    libgstreamer1.0-dev  libgstreamer-plugins-base1.0-dev  \
    gstreamer1.0-plugins-base  gstreamer1.0-plugins-good  \
    gstreamer1.0-plugins-bad  gstreamer1.0-plugins-ugly  \
    gstreamer1.0-libav  libgstrtspserver-1.0-dev  libges-1.0-dev \
    libglib2.0-dev
