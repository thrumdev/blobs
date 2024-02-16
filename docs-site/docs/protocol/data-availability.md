---
sidebar_position: 1
title: Data Availability
---

Ikura provides reliable and high-throughput Data Availability services. Before diving into exactly what this is, we'll take a quick detour through Rollup scaling solutions.

## Rollup Architecture

The family of Rollup scaling approaches is very broad, and includes systems such as Optimistic Rollups, ZK Rollups, Parachains, and more.

The core idea behind rollups is that new, Layer 2 blockchains can be bootstrapped from mature and secure Layer 1 blockchains simply by posting the transactions of the Layer 2 blockchains onto a Layer 1 chain as raw data. Any node scanning the Layer 1 blocks for Layer 2 transactions, and executing those Layer 2 transactions in order, will achieve the same outcome as any other node which does so. Therefore, the Layer 2 blockchain, or Rollup, is as secure as the Layer 1's transaction ordering guarantees. The terms Layer 2 and Rollup are typically used interchangeably, though the term Layer 2 also includes other scaling solutions which are not rollups.

Rollups are incredibly simple. The difficult part is convincing an agent (in this case, a user, smart contract, or another blockchain) what the state of the rollup is, without requiring that agent to execute every transaction from the rollup in order. Architectural distinctions between rollups emerge based on the **proving protocol** that is used to prove the current state of the rollup to an agent. Rollups may be designed to be compatible with multiple proving protocols simultaneously or may be optimized for a particular type of proving protocol.

All Rollup protocols follow roughly the same blueprint:
  1. **Sequencing**: Data which encodes one or more transactions is posted to a Layer 1 blockchain and inherits its transaction ordering guarantees.
  2. **Data Availability**: The transaction data, plus any other necessary data required for proving, is encoded, replicated, and made generally available to anyone on the internet to download. Available data cannot be withheld from someone who wishes to observe it.
  3. **Proving**: The effect of the transactions is proven, using the data from step (2). For example,
     * In ZK Rollups, a cryptographic, mathematical proof of the transactions' effects will be generated.
     * In Optimistic Rollups, anyone may claim that the transactions have certain effects, but their claim must either go unchallenged or be unsuccessfully challenged for a period of time. This submit-and-challenge game is itself a smart contract hosted on a blockchain.
     * In Parachains, Polkadot's validators re-execute the transactions and participate in a protocol to ensure consensus over the transactions' effects.
     * In Sovereign Rollups, no effort is made to prove the state of the rollup to an external agent, who is expected to execute all the transactions serially.

Data Availability serves an additional purpose, which is to ensure that submitters have all the necessary data to build the _next_ batch of transactions of the rollup, even if the previous submitters have withheld some data in an effort to monopolize transaction submission on the rollup.

Regardless of which proving mechanism is used, sequencing and data availability are common to all protocols under this blueprint.

Rollups use Ikura for both sequencing and Data Availability.

## Data Availability

Ikura is a wrapper over Polkadot's Data Availability system. Any data made available via Ikura is using the Polkadot validators under the hood. In this section, we'll explore how Polkadot makes data available.

A Blob consists of two things: a **namespace** and some **data**. The namespace is a kind of tag which allows for blobs to be filtered by an observer, and the data is just a bunch of bytes. To Ikura, the content of the blobs does not matter.

Ikura is a blockchain secured by Polkadot, where the blocks may contain Blob transactions.

Every block is erasure-coded, replicated, and distributed across Polkadot's entire validator set. The block is split up into pieces, with one piece for each Polkadot validator. Any set of more than one-third of the pieces is enough to recover the entire data.

### Example: The Path of a Blob
  1. First, the user signs a transaction containing the Blob and submits it to an Ikura node.
  2. Once the transaction has circulated through the network, a Ikura block authoring node bundles it into a block.
  3. The block authoring node submits the block to some assigned Polkadot validators.
  4. Those Polkadot validators approve the Ikura block and the Ikura block's header is placed into a Polkadot Relay Chain Block
  5. After this, the Ikura block, including all the transactions within it, is erasure-coded and split into redundant pieces, one for each Polkadot validator.
  6. Each validator attempts to fetch their piece over the p2p network. Polkadot requires that at least two-thirds of validators sign a statement that they have fetched and stored their piece, or the whole process must revert back to step (2).

