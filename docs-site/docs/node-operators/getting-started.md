---
title: Getting Started
---

## Releases

At the moment, there are no downloadable binary releases available of the Blobs node. We recommend building a binary from source.

## Node Architecture

Blobs is a standard Polkadot-SDK parachain node, which means that it actually runs two nodes:
  1. A node for the Parachain (Blobs) on Polkadot or Kusama
  2. A node for the Relay Chain, Polkadot or Kusama.

You can pass arguments to each one of these underlying nodes with the following approach:

```sh
./sugondat-node --arg-for-blobs --arg2-for-blobs -- --arg-for-relay --arg2-for-relay
```

## Hardware Requirements

For full nodes, we currently recommend:
  - 4 vCPUs
  - 8GB RAM
  - 512G Free Disk Space (1TB preferred)
  - Stable and fast internet connection. For collators, we recommend 500mbps.

The disk space requirements can be reduced by either
  1. Running with `--blocks-pruning 1000` in the arguments for both the parachain and relay chain arguments
  2. Running with `--blocks-pruning 1000 --relay-chain-light-client` in the parachain arguments.

Option (2) uses a relay chain light client rather than a full node. This may lead to data arriving at the node slightly later, as it will not follow the head of the relay chain as aggressively as a full node would. This option is not recommended for collators or RPC nodes.

## Building From Source

Building from source requires some familiarity with the terminal and your system's package manager.

First, ensure you have an up-to-date Rust installation. If not, visit https://rustup.rs .

Building from source requires a few packages to be installed on your system. On Debian or Ubuntu, these are:

```sh
apt install libssl-dev protobuf-compiler cmake make pkg-config llvm clang
```

On other Linux distributions or macOS, use your package manager's equivalents.

Building the node:
```bash
git clone https://github.com/thrumdev/blobs
cd blobs
cargo build --release -p sugondat-node
```

Running the node:
```bash
target/release/sugondat-node --chain sugondat-kusama
```

## Blobs and Storage Usage

Blobs can potentially use enormous amounts of disk space under heavy usage. This is because all historical blobs are stored within the blobchain's history. While the Polkadot and Kusama expunge ancient blobs after 24 hours, any full node of the blobchain will have all the blobs going back to the genesis, as well as all of the other block data.

To avoid this issue, run with `--blocks-pruning <number>`, where `number` is some relatively small value such as `1000` to avoid keeping all historical blobs.

However, there is still a need for archival nodes. The main reason is that many rollup SDKs do not have any form of p2p networking and nodes built with those platforms synchronize by downloading ancient blobs from the data availability layer's p2p network. Without someone keeping all ancient blocks, those SDKs
would unfortunately stop working. This is beyond the expectations of a data availability layer, as it is intractable to scale to 1Gbps while requiring data availability nodes to keep every historical blob. When all major rollup SDKs have introduced p2p synchronization, the potential storage burden on data availability full nodes will be reduced.
