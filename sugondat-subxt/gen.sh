#!/bin/bash

subxt codegen --url ws://localhost:9944/ | rustfmt --edition=2021 --emit=stdout > src/gen.rs
