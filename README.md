# Instruction to test demo-rollup

2 steps are required before:
+ build sugondat-chain
+ Adjust the PATHs to polkadot and sugondat-chain for zombienet

Now you can launch 2 polkadot validators and one sugondat-chain collator
``` sh
./zombienet.sh
```

Then launch the sugondat-shim with:
``` sh
cd sugondat-shim/
cargo run -p sugondat-shim -- serve --node-url=ws://localhost:9988/ --submit-dev-alice
``````

launch the demo rollup with:
``` sh
cd demo/demo-rollup
cargo run
```

execute the test
```
cd demo/demo-rollup
./test_create_token.sh
```

You should see at the end that a batch of two transactions was correctly pushed in the DA, fetched back and then executed in the rollup to create and mint 4000 new tokens

If you want to re-run zombienet and the demo rollup remember token

``` sh
rm -r zombienet
cd demo/demo-rollup
# clean the ledger db
make clean
```
