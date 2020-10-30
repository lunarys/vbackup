# =============================================
#  The base image with rsync would also be
#  available as lunarys/rsync-detect-renamed
#  but it does not support different archs
# =============================================
FROM debian:stable-slim as builder
ARG VERSION=3.1.3

RUN apt-get update && apt-get install -y build-essential wget

RUN wget https://download.samba.org/pub/rsync/src/rsync-${VERSION}.tar.gz \
	&& wget https://download.samba.org/pub/rsync/src/rsync-patches-${VERSION}.tar.gz \
	&& tar xzvf rsync-${VERSION}.tar.gz \
	&& tar xzvf rsync-patches-${VERSION}.tar.gz

WORKDIR rsync-${VERSION}

RUN patch -p1 <patches/detect-renamed.diff \
    && patch -p1 <patches/detect-renamed-lax.diff \
	&& ./configure \
	&& make \
	&& make install

# =============================================
#  Only the executables are required
# =============================================
FROM debian:stable-slim

COPY --from=builder /usr/local/bin/rsync* /usr/local/bin/

# Install rsync to get the requirements
RUN apt-get update &&  apt-get install -y rsync sshpass
