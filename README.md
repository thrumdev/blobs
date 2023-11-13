# Instruction to test demo-rollup

2 steps are required before:
+ build sugondat-chain
+ Adjust the PATHs to polkadot and sugondat-chain for zomienet

Now you can launch 2 polkadot validators and one sugondat-chain collator
``` sh
./zombienet.sh
```

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

# Usage

To launch the testnet you should use `zombienet`.

    # First, make sure that the binaries `polkadot` and `sugondat-node` are in your PATH.
    # If you're using a polkadot binary >=1.0.0 then you need also the binaries 
    # polkadot-execute-worker and polkadot-prepare-worker
    export PATH=/Users/pepyakin/dev/parity/polkadot/target/release/:$PATH
    export PATH=/Users/pepyakin/dev/parity/sugondat-chain/sugondat-chain/target/release/:$PATH

    # Then, launch the testnet.
    #
    # You will find logs in the `zombienet` directory.
    #
    # In order to relaunch the testnet, you should remove the `zombienet` directory.
    ./bin/zombienet-macos spawn -p native --dir zombienet testnet.toml

