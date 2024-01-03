FROM ubuntu:20.04 as builder

LABEL org.opencontainers.image.source=https://github.com/thrumdev/blobs

ARG RUSTC_VERSION=nightly-2023-10-16

ENV CARGO_INCREMENTAL=0
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/cargo_target
ENV RUSTFLAGS=""
ENV RUSTUP_HOME=/rustup
ENV WASM_BUILD_WORKSPACE_HINT=/sugondat
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

WORKDIR /sugondat
COPY . /sugondat

FROM builder AS builder-release

RUN --mount=type=cache,target=/cargo/git \
    --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo_target \
    $CARGO_HOME/bin/cargo build --locked --release -p sugondat-shim && \
        cp /cargo_target/release/sugondat-shim /usr/bin/sugondat-shim

FROM ubuntu:20.04

ENV TINI_VERSION v0.19.0
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini /tini
RUN chmod +x /tini

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates \
        curl

COPY --from=builder-release /usr/bin/sugondat-shim /usr/bin/sugondat-shim

EXPOSE 10995

ENTRYPOINT ["/tini", "--", "/usr/bin/sugondat-shim"]
CMD ["serve", "-p", "10995"]
