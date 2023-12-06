FROM ubuntu:20.04 as builder

LABEL org.opencontainers.image.source=https://github.com/pepyakin/bfb

ARG RUSTC_VERSION=nightly-2023-10-16

ENV CARGO_INCREMENTAL=0
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/cargo_target
ENV RUSTFLAGS=""
ENV RUSTUP_HOME=/rustup

RUN mkdir -p /cargo
RUN mkdir -p /cargo_target
RUN mkdir -p /rustup

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
        make

WORKDIR /sugondat
COPY . /sugondat

RUN \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUSTC_VERSION
RUN $CARGO_HOME/bin/rustup target add wasm32-unknown-unknown

RUN $CARGO_HOME/bin/cargo build -p sugondat-node --locked --release

# FROM docker.io/library/ubuntu:20.04
# COPY --from=builder /sugondat/target/release/sugondat-node /usr/local/bin

#USER sugondat
# EXPOSE 30333 9933 9944 9615
# VOLUME ["/data"]
# ENTRYPOINT [ "/usr/local/bin/sugondat-node" ]
