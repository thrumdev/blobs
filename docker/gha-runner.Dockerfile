# An image that acts as the base image for the GHA runner running in sysbox.
#
# Build: docker build -t ghcr.io/thrumdev/gha-runner -f docker/gha-runner.Dockerfile .
# Push: docker push ghcr.io/thrumdev/gha-runner

FROM rodnymolina588/gha-sysbox-runner@sha256:d10a36f2da30aa0df71d1ac062cc79fc5114eec7b6ae8a0c42cadf568e6eefa8

ARG RUSTC_VERSION=stable

LABEL org.opencontainers.image.source=https://github.com/thrumdev/blobs

ENV CARGO_INCREMENTAL=0
ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/cargo_target
ENV RUSTFLAGS=""
ENV RUSTUP_HOME=/rustup

RUN curl -sL https://deb.nodesource.com/setup_20.x | bash -
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
        docker-compose-plugin \
        nodejs \
        multitail

RUN \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain $RUSTC_VERSION
ENV PATH=$CARGO_HOME/bin:$PATH

RUN rustup target add wasm32-unknown-unknown
RUN rustup component add rust-src

# Install cargo binstall, using it install cargo-risczero, and using it install risc0 toolchain.
RUN curl \
    -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh \
    | bash
RUN cargo binstall --no-confirm --no-symlinks cargo-risczero
RUN cargo risczero install

# Install Zombienet and copy Polkadot binaries, which are all required for xtask tests
RUN npm install -g @zombienet/cli

COPY --from=parity/polkadot:v1.6.0 /usr/bin/polkadot /usr/bin/
COPY --from=parity/polkadot:v1.6.0 /usr/lib/polkadot/polkadot-prepare-worker /usr/bin/
COPY --from=parity/polkadot:v1.6.0 /usr/lib/polkadot/polkadot-execute-worker /usr/bin/
