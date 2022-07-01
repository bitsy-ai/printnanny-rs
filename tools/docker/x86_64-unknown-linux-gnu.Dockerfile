FROM ghcr.io/cross-rs/x86_64-unknown-linux-gnu:edge
RUN yum install -y openssl-devel \
    libgstreamer1.0-dev   libgstreamer-plugins-base1.0-dev   \
    gstreamer1.0-plugins-base   gstreamer1.0-plugins-good   \
    gstreamer1.0-plugins-bad   gstreamer1.0-plugins-ugly   \
    gstreamer1.0-libav   libgstrtspserver-1.0-dev   libges-1.0-dev  

