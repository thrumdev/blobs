---
sidebar_position: 1
---

# Getting Started

## Blobs

The Blobs project exposes the **data availability** capabilities of [Polkadot](https://polkadot.network) and [Kusama](https://kusama.network) for general use. Use-cases include rollups and inscriptions.

The Blobs codebase is located at https://github.com/thrumdev/blobs. There is a live parachain on Kusama with Parachain ID 3338 running the Blobs runtime.

Blobs enables users to submit arbitrary data to the chain and receive guarantees about the availability of that data. Namely:
  1. The data can be fetched from the Polkadot/Kusama validator set for up to 24 hours after submission and cannot be withheld.
  2. A commitment to the data's availability is stored within the blobchain and used as a proof of guarantee (1) to computer programs, such as smart contracts or Zero-Knowledge circuits.

Data Availability is a key component of Layer-2 scaling approaches, and is already part of Polkadot and Kusama for use in securing Parachains. Blobs will bring this capability out to much broader markets. 

Blobs makes a **relay-chain token utility commitment** now and forever. Submitting blobs will always make use of the DOT token on Polkadot and the KSM token on Kusama, as this is the approach with the least user friction.

## Integrations

Blobs supports a variety of rollup SDKs out of the box.
  - [x] Rollkit
  - [x] Sovereign SDK
  - [ ] OP Stack
  - [ ] Polygon ZK-EVM