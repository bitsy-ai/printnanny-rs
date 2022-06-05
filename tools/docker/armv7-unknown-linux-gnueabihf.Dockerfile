FROM rustembedded/cross:armv7-unknown-linux-gnueabihf
RUN dpkg --add-architecture armv7
RUN apt-get update && apt-get install --assume-yes libssl-dev:armv7
