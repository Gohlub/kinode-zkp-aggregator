# Proof Aggregator Template

This contract is based on a template for writing a contract that uses verification of [SP1](https://github.com/succinctlabs/sp1) PlonK proofs onchain using the [SP1VerifierGateway](https://github.com/succinctlabs/sp1-contracts/blob/main/contracts/src/SP1VerifierGateway.sol). It was appropriated for the needs of [Uncentered Systems](https://uncentered.systems/).

## Requirements

- [Foundry](https://book.getfoundry.sh/getting-started/installation)

## Test

```sh
forge test -v
```

## Deployment

#### Step 1: Set the `VERIFIER` environment variable

Find the address of the `verifer` to use from the [contract addresses](https://docs.succinct.xyz/verification/onchain/contract-addresses.html) list for the chain you are deploying to. Set it to the `VERIFIER` environment variable, for example:
This is the verifier of the Groth16 proof on Optimism mainnet.
```sh
VERIFIER=0x397A5f7f3dBd538f23DE225B51f532c34448dA9B
```

Note: you can use either the [SP1VerifierGateway](https://github.com/succinctlabs/sp1-contracts/blob/main/contracts/src/SP1VerifierGateway.sol) or a specific version, but it is highly recommended to use the gateway as this will allow you to use different versions of SP1.

#### Step 2: Set the `PROGRAM_VKEY` environment variable
To generate the elf (and the verification key), you will need to install Succinct Labs' `sp1` toolchain. You can find the instructions [here](https://github.com/succinctlabs/sp1). Then, to generate the elf, run:
```sh
cd /aggregator_program
cargo prove build
cargo prove vkey --elf <ELF_PATH>
```
This will generate the verification key you will then set in the environment variables for the contract deployment.

For example:
```sh
PROGRAM_VKEY=0x00620892344c310c32a74bf0807a5c043964264e4f37c96a10ad12b5c9214e0e
```

#### Step 3: Deploy the contract

Fill out the rest of the details needed for deployment:

```sh
CURRENT_RPC_URL=...
```

```sh
WALLET_PRIVATE_KEY=...
```

Then deploy the contract to the chain:

```sh
forge create src/SP1AggregateVerifier.sol:SP1AggregateVerifier --rpc-url $CURRENT_RPC_URL --private-key $WALLET_PRIVATE_KEY --constructor-args $VERIFIER $PROGRAM_VKEY
```

It can also be a good idea to verify the contract when you deploy, in which case you would also need to set `ETHERSCAN_API_KEY`:

```sh
forge create src/SP1AggregateVerifier.sol:SP1AggregateVerifier --rpc-url $CURRENT_RPC_URL --private-key $PRIVATE_KEY --constructor-args $VERIFIER $PROGRAM_VKEY --verify --verifier etherscan --etherscan-api-key $ETHERSCAN_API_KEY
```

To interact with the contract, refer to the terminal debug commands in the main [README](../README.md).

## Proof Construction

For examples of how to construct proofs, refer to the fixture file in `src/fixtures/groth16-offchain.json`. This file contains the inputs needed to reconstruct the merkle tree:

```json
{
  "verificationKeys": [...],
  "publicValues": [...],
  "leafIndices": [...]
}
```

To construct proofs using these values:

1. Each leaf in the merkle tree is constructed by:
   - Concatenating the verification key and public values for each proof
   - Hashing the concatenated result using SHA256

2. The program demonstrates this in the `commit_proof_pairs` function ([../program/src/main.rs#L16](../program/src/main.rs)):

```rust
let leaves: Vec<[u8; 32]> = vkeys
    .iter()
    .zip(committed_values.iter())
    .map(|(vkey, value)| {
        let concat = [&words_to_bytes_le(vkey)[..], value].concat();
        MerkleSha256::hash(&concat)
    })
    .collect();
```

3. Once you have the leaves, you can:
   - Construct the full merkle tree using `MerkleTree::from_leaves()`
   - Generate proofs for specific indices using `merkle_tree.proof(&indices)`
   - Verify proofs using `proof.verify(root, leaf_indices, leaf_hashes, total_leaves_count)`

You can reference the [rs_merkle documentation](https://docs.rs/rs-merkle/latest/rs_merkle/index.html) for more information.
