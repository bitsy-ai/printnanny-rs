FROM rustembedded/cross:arm7l-unknown-linux-gnu-0.2.1

RUN dpkg --add-architecture arm7l && \
    apt-get update && \
    apt-get install -y libssl-dev