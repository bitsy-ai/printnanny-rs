FROM ubuntu:22.04

ARG DEBIAN_FRONTEND=noninteractive

COPY docker/common.sh docker/lib.sh /
RUN /common.sh

COPY docker/cmake.sh /
RUN /cmake.sh

COPY docker/xargo.sh /
RUN /xargo.sh

COPY docker/qemu.sh /
RUN /qemu.sh x86_64 softmmu

COPY docker/dropbear.sh /
RUN /dropbear.sh

COPY docker/linux-image.sh /
RUN /linux-image.sh x86_64

COPY docker/linux-runner /

ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER="/linux-runner x86_64"
RUN apt-get install -y software-properties-common

RUN apt-add-repository ppa:nnstreamer
RUN apt-get update -y && apt-get install -y --upgrade \
    libssl-dev \
    libgstreamer1.0-dev  libgstreamer-plugins-base1.0-dev  \
    gstreamer1.0-plugins-base  gstreamer1.0-plugins-good  \
    gstreamer1.0-plugins-bad  gstreamer1.0-plugins-ugly  \
    gstreamer1.0-libav  libgstrtspserver-1.0-dev  libges-1.0-dev \
    libglib2.0-dev \
    nnstreamer \
    nnstreamer-tensorflow2-lite \
    nnstreamer-dev

# nodejs is required to build printnanny-dash package
RUN curl -fsSL https://deb.nodesource.com/setup_16.x | bash - && apt-get install -y nodejs build-essential gcc g++ make
