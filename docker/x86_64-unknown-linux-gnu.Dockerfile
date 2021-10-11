FROM rustembedded/cross:arm7l-unknown-linux-gnu-0.2.1

RUN apt-get update && \
    apt-get install -y libssl-dev