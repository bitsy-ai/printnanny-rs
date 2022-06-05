FROM rustembedded/cross:x86_64-unknown-linux-gnu-0.2.1
RUN dpkg --add-architecture amd64
RUN apt-get update && apt-get install --assume-yes libssl-dev:amd64
