FROM ubuntu:20.04

# TODO: update this
LABEL org.opencontainers.image.source=https://github.com/pepyakin/bfb

ARG RUSTC_VERSION=nightly-2023-10-16

ENV CARGO_INCREMENTAL=0
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/cargo_target
ENV RUSTFLAGS=""
ENV RUSTUP_HOME=/rustup

RUN mkdir -p /cargo && \
    mkdir -p /cargo_target && \
    mkdir -p /rustup

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates \
        protobuf-compiler \
        curl \
        git \
        llvm \
        clang \
        cmake \
        make \
        libssl-dev \
        pkg-config

RUN \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUSTC_VERSION
RUN $CARGO_HOME/bin/rustup target add wasm32-unknown-unknown

RUN $CARGO_HOME/bin/cargo install cargo-risczero
RUN $CARGO_HOME/bin/cargo risczero build-toolchain

WORKDIR /sugondat
COPY . /sugondat

ENV CONSTANTS_MANIFEST=/sugondat/demo/sovereign/constants.json
RUN cd demo/sovereign && $CARGO_HOME/bin/cargo build --locked --release
