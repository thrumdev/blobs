FROM ubuntu:20.04 as builder

ARG RUSTC_VERSION=nightly-2023-10-16

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
        make && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUSTC_VERSION

RUN /root/.cargo/bin/rustup target add wasm32-unknown-unknown

WORKDIR /sugondat
COPY . /sugondat

RUN /root/.cargo/bin/cargo build -p sugondat-node --locked --release

FROM docker.io/library/ubuntu:20.04

COPY --from=builder /sugondat/target/release/sugondat-node /usr/local/bin

USER sugondat
EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]
ENTRYPOINT [ "/usr/local/bin/sugondat-node" ]
