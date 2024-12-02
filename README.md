# Kinode Zero Knowledge Proof Aggregation Provider

## Dev setup
Order matters. The zkp-aggregator process assumes the contract is deployed and live. The instructions to deploy the contract are in the [contracts README](./contracts/README.md).

To run the aggregator program, you must install the process to a local live node with:

```sh
kit bs --port <NODE_PORT>
```
For the WS client to initialize properly, the aggregator process must be running.
After the process is running, do:
```sh
cd /zkp-aggregator-ext
cargo run
```
Both the client and the node terminal will print messages when a successful connection is made.

## Terminal Debug Commands
Note that I have left some pre-created proofs in the `zkp-aggregator-ext/dummy_proofs` directory. I will leave those for testing purposes until we have a more permanent solution generating these proofs on nodes. The aggregator does nothing more than ingest proofs, aggregate them, and submit them to the contract. You can utilize the terminal debugger to inspect the proofs and the state of the aggregator.
You can do this with in the node terminal:
```sh
m our@zkp-aggregator:zkp-aggregator:punctumfix.os <COMMAND>
```

You can utilize the terminal debugger to inspect the proofs and the state of the aggregator. Here are the available commands:

- `print_state`: Prints the current state of the aggregator.
- `current_epoch`: Prints the current epoch and its state.
- `list_epochs`: Lists all the epochs.
- `print_epoch:<epoch_number>`: Prints the state of a specific epoch.
- `insert_dummy_proofs`: Inserts dummy proofs into the state. (This is proxied by the WS client since proof objects are not loadable into the kinode process directly.)
- `request_aggregate_proofs`: Requests the aggregation of proofs from the state and sends them via WebSocket. (The aggregation is actually handled by the timer module, this just triggers the process if needed.)
- `send_to_chain`: Sends the aggregated proof to the blockchain. (Similar to the above, this just triggers the process if needed.)

# Outline
- `/contracts` - contains the contract code and instructions to deploy it.
- `/zkp-aggregator` - contains the aggregator process.
- `/zkp-aggregator-ext` - contains the Websocket client.
- `/shared_types` - contains the shared types between the aggregator and client.
- `/aggregator_program` - contains the aggregator program. The binary if this program is sent to the prover network along with the inputs.
- `/elf` - contains the ELF binary of the aggregator program. Since every program will have a different verification key, it is suggested that you generate your own. You will need to set the verification key of your program since it deployed along with the contract (it does not change depending on input, just the program).

### Note on ZKP and Kinode
Succinct's ZKP *sp1* toolchain and SDK are currently incompatible with WASM due to complicated dependacy conflicts. To that point, interfacing with the SDK is done mostly through a Websocket client.

For using the prover network, refer to the [Succinct Labs Prover Network Documentation](https://docs.succinct.xyz/generating-proofs/prover-network.html) for setting up your API Key, and make sure to set the `SP1_PRIVATE_KEY` environment variable.