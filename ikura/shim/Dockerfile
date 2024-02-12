FROM ubuntu:20.04 as builder

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

WORKDIR /ikura
COPY . /ikura

FROM builder AS builder-release

RUN --mount=type=cache,id=ikura-shim,target=/cargo/git \
    --mount=type=cache,id=ikura-shim,target=/cargo/registry \
    --mount=type=cache,id=ikura-shim,target=/cargo_target \
    $CARGO_HOME/bin/cargo build --locked --release -p ikura-shim && \
        cp /cargo_target/release/ikura-shim /usr/bin/ikura-shim

FROM ubuntu:20.04

ENV TINI_VERSION v0.19.0
ARG TARGETARCH
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-${TARGETARCH} /tini
RUN chmod +x /tini

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates \
        curl

COPY --from=builder-release /usr/bin/ikura-shim /usr/bin/ikura-shim

EXPOSE 10995

ENTRYPOINT ["/tini", "--", "/usr/bin/ikura-shim"]
CMD ["serve", "-p", "10995"]
