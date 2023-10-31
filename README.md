# Usage

To launch the testnet you should use `zombienet`.

    # First, make sure that the binaries `polkadot` and `sugondat-node` are in your PATH.
    export PATH=/Users/pepyakin/dev/parity/polkadot/target/release/:$PATH
    export PATH=/Users/pepyakin/dev/parity/sugondat-chain/sugondat-chain/target/release/:$PATH

    # Then, launch the testnet.
    #
    # You will find logs in the `zombienet` directory.
    #
    # In order to relaunch the testnet, you should remove the `zombienet` directory.
    ./bin/zombienet-macos spawn -p native --dir zombienet testnet.toml

