#!/bin/bash

forge build
# Extract the 'abi' field from the source JSON and write it to the target file
cp out/SP1AggregateVerifier.sol/SP1AggregateVerifier.json ../zkp-aggregator/abi/SP1AggregateVerifier.json
echo "ABI for SP1AggregateVerifier updated successfully."
