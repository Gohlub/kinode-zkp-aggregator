use crate::caller::Caller;
use crate::CURRENT_CHAIN_ID;
use alloy_primitives::U256;
use alloy_sol_types::{sol, SolCall};
use kinode_process_lib::kiprintln;
use serde::{Deserialize, Serialize};
use shared_types::AggregationOutput;
/* ABI import */
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    #[derive(Debug, Deserialize, Serialize)]
    SP1AggregateVerifier,
    "abi/SP1AggregateVerifier.json"
);

pub struct ContractCaller {
    pub caller: Caller,
    pub contract_address: String,
}

impl ContractCaller {
    pub fn verify_aggregate_proof_and_update_root(
        &self,
        output: AggregationOutput,
    ) -> anyhow::Result<()> {
        kiprintln!("Starting transaction...");

        let public_values_hex = output.proof.public_values.to_vec();
        let proof_bytes_hex = output.proof.bytes();

        let call = SP1AggregateVerifier::verifyAggregateProofAndUpdateRootCall {
            _publicValues: public_values_hex.into(),
            _proofBytes: proof_bytes_hex.into(),
        }
        .abi_encode();

        match self.caller.send_tx(
            call,
            &self.contract_address,
            2_000_000,
            1_000_000,
            100_000,
            U256::from(0),
            *CURRENT_CHAIN_ID,
        ) {
            Ok((tx_hash, _nonce)) => {
                kiprintln!("Transaction sent successfully! Hash: {}", tx_hash);
                Ok(())
            }
            Err(e) => {
                kiprintln!("Transaction failed with error: {:?}", e);
                Err(anyhow::anyhow!(
                    "Error verifying proof and updating root: {:?}",
                    e
                ))
            }
        }
    }
}
