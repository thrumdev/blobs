FROM ubuntu:20.04 as builder

LABEL org.opencontainers.image.source=https://github.com/thrumdev/blobs

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
        pkg-config \
        ninja-build \
        python3

RUN \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUSTC_VERSION

RUN curl \
        -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
        | bash

# Mount caches because it will build on unsupported platforms.
RUN \
    --mount=type=cache,target=/cargo/git \
    --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo_target \
    $CARGO_HOME/bin/cargo binstall --no-confirm --no-symlinks cargo-risczero

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends python3 && \
    ln -s /usr/bin/python3 /usr/bin/python

RUN \
    --mount=type=cache,target=/cargo/git \
    --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo_target \
    --mount=type=cache,target=/root/.risc0/rust \
    $CARGO_HOME/bin/cargo risczero install \
        || ( \
            $CARGO_HOME/bin/cargo risczero build-toolchain \
            && rm /rustup/toolchains/risc0 \
            && cp -r /root/.risc0/rust/build/aarch64-unknown-linux-gnu/stage2 /rustup/toolchains/risc0 \
        )
