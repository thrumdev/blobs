FROM ghcr.io/thrumdev/risczero:latest as risczero

FROM ubuntu:20.04 as builder

LABEL org.opencontainers.image.source=https://github.com/thrumdev/blobs

ARG RUSTC_VERSION=nightly-2023-10-16

ENV CARGO_INCREMENTAL=0
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/cargo_target
ENV RUSTFLAGS="-Cdebuginfo=0"
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

COPY --from=risczero $CARGO_HOME/bin/cargo-risczero $CARGO_HOME/bin/cargo-risczero
COPY --from=risczero $CARGO_HOME/bin/r0vm $CARGO_HOME/bin/r0vm
COPY --from=risczero /rustup/toolchains/risc0 $RUSTUP_HOME/toolchains/risc0

WORKDIR /ikura
COPY . /ikura

ENV CONSTANTS_MANIFEST=/ikura/demo/sovereign/constants.json
RUN \
    --mount=type=cache,id=demo-sovereign,target=/cargo/git \
    --mount=type=cache,id=demo-sovereign,target=/cargo/registry \
    --mount=type=cache,id=demo-sovereign,target=/cargo_target \
    cd demo/sovereign \
        && $CARGO_HOME/bin/cargo build --release --locked \
        && cp $CARGO_TARGET_DIR/release/sov-demo-rollup /usr/bin/sov-demo-rollup \
        && cp $CARGO_TARGET_DIR/release/sov-cli /usr/bin/sov-cli

FROM ubuntu:20.04 as prod

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        libssl-dev \
        pkg-config

ENV TINI_VERSION v0.19.0
ARG TARGETARCH
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-${TARGETARCH} /tini
RUN chmod +x /tini

COPY --from=builder /usr/bin/sov-demo-rollup /usr/bin/sov-demo-rollup
COPY --from=builder /usr/bin/sov-cli /usr/bin/sov-cli

COPY ./demo/sovereign /ikura/demo/sovereign
COPY ./demo/sovereign/docker/rollup_config.docker.toml /ikura/demo/sovereign/demo-rollup/rollup_config.toml
WORKDIR /ikura/demo/sovereign/demo-rollup

EXPOSE 12345

ENTRYPOINT ["/tini", "--", "/usr/bin/sov-demo-rollup"]
