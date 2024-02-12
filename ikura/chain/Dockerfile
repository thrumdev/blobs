# HACK: Take note that this Dockerfile is meant to be used on x86-64 and apple silicon. However,
# we have to use `--platform=amd64` even on macs relying on rosetta 2 to run the code. The reason
# for that is zombienet requires a polkadot binary. Sadly, Polkadot is not packaged as a multi-
# arch binary.

FROM --platform=amd64 ubuntu:20.04 as builder

LABEL org.opencontainers.image.source=https://github.com/thrumdev/blobs

ARG RUSTC_VERSION=nightly-2023-10-16

ENV CARGO_INCREMENTAL=0
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/cargo_target
ENV RUSTFLAGS=""
ENV RUSTUP_HOME=/rustup
ENV WASM_BUILD_WORKSPACE_HINT=/ikura
# Force fetching git deps with git CLI instead of libgit2 to workaround build failures.
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

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
        make

RUN \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUSTC_VERSION
RUN $CARGO_HOME/bin/rustup target add wasm32-unknown-unknown
RUN $CARGO_HOME/bin/rustup component add rust-src

WORKDIR /ikura
COPY . /ikura

FROM --platform=amd64 builder AS builder-release

RUN --mount=type=cache,id=ikura-chain,target=/cargo/git \
    --mount=type=cache,id=ikura-chain,target=/cargo/registry \
    --mount=type=cache,id=ikura-chain,target=/cargo_target \
    $CARGO_HOME/bin/cargo build --locked --release -p ikura-node && \
        cp /cargo_target/release/ikura-node /usr/bin/ikura-node

FROM --platform=amd64 ubuntu:20.04 as prod

ENV TINI_VERSION v0.19.0
# Hardcoded to amd64. See the comment at the top of this file.
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-amd64 /tini
RUN chmod +x /tini

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates \
        curl

COPY --from=builder-release /usr/bin/ikura-node /usr/bin/ikura-node

ENTRYPOINT ["/tini", "--", "/usr/bin/ikura-node"]

# This target supplements ikura-node with all the sufficient components to spawn a full local
# testnet (zombienet).
FROM --platform=amd64 prod as zombienet

RUN curl -sL https://deb.nodesource.com/setup_20.x | bash -
RUN apt-get install -y nodejs multitail
RUN npm install -g @zombienet/cli

COPY --from=parity/polkadot:v1.6.0 /usr/bin/polkadot /usr/bin/
COPY --from=parity/polkadot:v1.6.0 /usr/lib/polkadot/polkadot-prepare-worker /usr/bin/
COPY --from=parity/polkadot:v1.6.0 /usr/lib/polkadot/polkadot-execute-worker /usr/bin/

COPY ./testnet.toml /testnet.toml

EXPOSE 9988

VOLUME /zombienet
ENTRYPOINT ["/tini", "--", "zombienet", "spawn", "--provider=native", "-d/zombienet/data", "/testnet.toml"]
