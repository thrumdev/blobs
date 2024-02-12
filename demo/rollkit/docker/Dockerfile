FROM golang:1.21 as builder

LABEL org.opencontainers.image.source=https://github.com/thrumdev/blobs

RUN \
    apt-get update && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        ca-certificates \
        curl

# Install ignite CLI
RUN curl https://get.ignite.com/cli@v0.27.1! | bash

# it seems that `go work` is broken in docker. Let's just copy the ikura package.
COPY ./demo/rollkit /demo-rollkit
COPY ./adapters/rollkit /demo-rollkit/ikura
# Patch the source file
RUN sed -i 's/\"ikura\"/\"gm\/ikura\"/' /demo-rollkit/cmd/gmd/cmd/root.go
RUN rm /demo-rollkit/ikura/go.*
WORKDIR /demo-rollkit

RUN --mount=type=cache,target=/go/pkg \
    --mount=type=cache,target=/root/.cache/go-build \
    go mod download && \
        go mod verify && \
        ignite chain build

FROM golang:1.21 AS prod

ENV TINI_VERSION v0.19.0
ARG TARGETARCH
ADD https://github.com/krallin/tini/releases/download/${TINI_VERSION}/tini-${TARGETARCH} /tini
RUN chmod +x /tini

COPY --from=builder /go/bin/gmd /usr/local/bin/gmd
COPY ./demo/rollkit/init-local.sh /root/init-local.sh
WORKDIR /root
ENTRYPOINT ["/tini", "/root/init-local.sh"]
