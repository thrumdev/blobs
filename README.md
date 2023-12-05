# Sugondat

## Project Structure

<pre>
<a href=".">sugondat</a>: The sugondat monorepo.
├──<a href="./adapters">adapters</a>: Adapters for various rollup development kits (RDK).
│   ├── <a href="./adapters/rollkit">rollkit</a>: An adapter connecting Rollkit to Sugondat
│   ├── <a href="./adapters/sovereign">sovereign</a>: An adapter connecting Sovereign to Sugondat.
├──<a href="./ci">ci</a>: All CI & QA related tools.
├──<a href="./demo">demo</a>: Projects showcasing integration of RDKs with Sugondat.
│   ├── <a href="./demo/rollkit">rollkit</a>: Rollkit's GM rollup.
│   ├── <a href="./demo/sovereign">sovereign</a>: Sovereign Demo Rollup.
├──<a href="./sugondat-chain">sugondat-chain</a>: Implementation of sugondat parachain.
├──<a href="./sugondat-shim">sugondat-shim</a>: Shim between sugondat parachain RPC and RDK adapters.
├──<a href="./sugondat-subxt">sugondat-subxt</a>: Bindings to Sugondat RPC.
</pre>

## Running Demos

### Prerequisites

In general you need to have the following components running:

build sugondat-chain:

``` sh
cd sugondat-chain
cargo build --release
```

Make sure that zombienet binary and sugondat-chain binary are in your PATH.

Now you can launch 2 polkadot validators and one sugondat-chain collator

``` sh
./zombienet.sh
```

Then launch the sugondat-shim with:

``` sh
cd sugondat-shim/
cargo run -p sugondat-shim -- serve --submit-dev-alice
``````

### Sovereign Demo

launch the demo rollup with:

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

If you want to re-run zombienet and the demo rollup remember token

``` sh
rm -r zombienet
cd demo/sovereign/demo-rollup
# clean the ledger db
make clean
```

### Rollkit Demo

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
gmd unsafe-reset-all
```
