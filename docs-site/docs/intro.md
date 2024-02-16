---
sidebar_position: 1
title: Introduction

---

## Ikura

Ikura exposes **sequencing** and **data availability** capabilities of [Polkadot](https://polkadot.network) and [Kusama](https://kusama.network) for general use. Use-cases include rollups and inscriptions.

The Ikura codebase is located at https://github.com/thrumdev/blobs.

In this documentation site, we'll often use the term Polkadot to refer to the Polkadot Relay Chain - the hub chain which provides security for everything running on Polkadot. Kusama runs on the same technology as Polkadot, so the Kusama version of Ikura (Gondatsu) works identically to the Polkadot version, just with a different network. You can mentally substitute "Polkadot" for "Kusama" when thinking about the Kusama version of Ikura.

Ikura enables users to submit arbitrary data to the blockchain chain and receive guarantees about the availability of that data, as well as proofs of the order in which data were submitted. Those guarantees are:
  1. The data can be fetched from the Polkadot/Kusama validator set for up to 24 hours after submission and cannot be withheld.
  2. A commitment to the data's availability is stored within the blobchain and used as a proof of guarantee (1) to computer programs, such as smart contracts or Zero-Knowledge circuits.

Data Availability is a key component of Layer-2 scaling approaches, and is already part of Polkadot and Kusama for use in securing Parachains. Ikura will bring this capability to use-cases beyond parachains with a minimal interface.

Ikura makes a **relay-chain token utility commitment** now and forever. Submitting blobs will always make use of the DOT token on Polkadot and the KSM token on Kusama, as this is the approach with the least user friction.

## Gondatsu

The version of Ikura targeting the Kusama network is **Gondatsu**. It has Parachain ID 3338 running. We currently do not make any long-term stability commitment for Gondatsu. Gondatsu currently is _centrally controlled_ with a sudo (admin) key. It is experimental.

## Integrations

Ikura supports a variety of rollup SDKs out of the box.
  - [x] Rollkit
  - [x] Sovereign SDK
  - [ ] OP Stack
  - [ ] Polygon ZK-EVM
