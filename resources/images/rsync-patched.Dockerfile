# =============================================
#  The base image with rsync would also be
#  available as lunarys/rsync-detect-renamed
#  but it does not support different archs
# =============================================
FROM debian:testing-slim as builder
ARG VERSION=3.2.3

RUN apt-get update && apt-get install -y \
	gcc g++ gawk autoconf automake python3-cmarkgfm \
	acl libacl1-dev \
	attr libattr1-dev \
	libxxhash-dev \
	libzstd-dev \
	liblz4-dev \
	libssl-dev \
	build-essential \
	wget

RUN wget https://download.samba.org/pub/rsync/rsync-${VERSION}.tar.gz \
	&& wget https://download.samba.org/pub/rsync/rsync-patches-${VERSION}.tar.gz \
	&& tar xzvf rsync-${VERSION}.tar.gz \
	&& tar xzvf rsync-patches-${VERSION}.tar.gz

WORKDIR rsync-${VERSION}

RUN patch -p1 <patches/detect-renamed.diff \
	&& ./configure \
	&& make \
	&& make install

# =============================================
#  Only the executables are required
# =============================================
FROM debian:testing-slim

COPY --from=builder /usr/local/bin/rsync* /usr/local/bin/

# Install rsync to get the requirements
RUN apt-get update &&  apt-get install -y rsync sshpass
