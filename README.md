# Ikura

Blobchains on Polkadot and Kusama

## Project Structure

<pre>
<a href=".">blobs</a>: The Ikura monorepo.
├──<a href="./adapters">adapters</a>: Adapters for various rollup development kits (RDK).
│   ├── <a href="./adapters/sovereign">sovereign</a>: An adapter connecting Sovereign to Ikura.
├──<a href="./demo">demo</a>: Projects showcasing integration of RDKs with Ikura.
│   ├── <a href="./demo/rollkit">rollkit</a>: Rollkit's GM rollup.
│   ├── <a href="./demo/sovereign">sovereign</a>: Sovereign Demo Rollup.
|--<a href="./docs-site">docs-site</a>: Documentation site source, using Docusaurus.
|──<a href="./ikura">ikura</a>: Ikura source code.
│   ├──<a href="./ikura/chain">ikura-chain</a>: Implementation of the Ikura parachain.
│   ├──<a href="./ikura/nmt">ikura-nmt</a>: Namespaced Merkle Trie definitions.
│   ├──<a href="./ikura/serde-util">ikura-serde-util</a>: Various utilities for serde.
│   ├──<a href="./ikura/shim">ikura-shim</a>: Shim between ikura parachain RPC and RDK adapters.
│   ├──<a href="./ikura/subxt-autogen">ikura-subxt</a>: Bindings to Ikura RPC.
</pre>

## Running Demos

### Prerequisites

In general you need to have the following components running:

build ikura-chain:

``` sh
cd ikura/chain
cargo build --release
```

Make sure that zombienet binary and ikura-chain binary are in your PATH.

Now you can launch 2 polkadot validators and one ikura-chain collator

``` sh
./zombienet.sh
```

### Sovereign Demo

launch the ikura-shim with:

``` sh
cargo run -p ikura-shim -- serve sov --submit-dev-alice
``````

then launch the demo rollup with:

``` sh
cd demo/sovereign/demo-rollup
cargo run
```

execute the test

```
cd demo/sovereign/demo-rollup
./test_create_token.sh
```

You should see at the end that a batch of two transactions was correctly pushed in the DA, fetched back and then executed in the rollup to create and mint 4000 new tokens

If you want to rerun the demo, you need to reset zombienet and the demo-rollup

``` sh
rm -r zombienet
cd demo/sovereign/demo-rollup
# clean the ledger db
make clean
```

### Rollkit Demo

```sh
cargo run -p ikura-shim -- serve rollkit --port 26650 --submit-dev-alice --namespace 01
```

[Original instructions](https://rollkit.dev/tutorials/gm-world) should work. Make sure to check them
out for prerequisites and other details. Below is a quick summary for reference.

Make sure that go bin folder is in path.

```sh
export PATH=$PATH:$(go env GOPATH)/bin
```

go to the rollkit demo folder and launch ./init-local.sh

``` sh
cd demo/rollkit
./init-local.sh
```

Then use the following command to get the demo keys:

``` sh
gmd keys list --keyring-backend test
```

save them into environment variables:

``` sh
export KEY1=gm1sa3xvrkvwhktjppxzaayst7s7z4ar06rk37jq7
export KEY2=gm13nf52x452c527nycahthqq4y9phcmvat9nejl2
```

then you can send a transaction and check the results:

```sh
# it will ask for confirmation, type "y" and press enter
gmd tx bank send $KEY1 $KEY2 42069stake --keyring-backend test \
--node tcp://127.0.0.1:36657


gmd query bank balances $KEY2 --node tcp://127.0.0.1:36657
gmd query bank balances $KEY1 --node tcp://127.0.0.1:36657
```

If you see the amounts:

```
10000000000000000000042069
9999999999999999999957931
```

that means it worked!

To reset the chain

```sh
rm -r zombienet
cd demo/rollkit
gmd tendermint unsafe-reset-all
```
