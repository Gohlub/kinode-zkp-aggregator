use serde::{Deserialize, Serialize};
use sp1_sdk::{SP1ProofWithPublicValues, SP1VerifyingKey};

#[derive(Serialize, Deserialize, Clone)]
pub struct AggregationInput {
    pub proof: SP1ProofWithPublicValues,
    pub vk: SP1VerifyingKey,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AggregationOutput {
    pub proof: SP1ProofWithPublicValues,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DummyProofInsert {
    pub proofs: Vec<AggregationInput>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum WsMessage {
    Aggregation(AggregationOutput),
    DummyProof(DummyProofInsert),
}

impl std::fmt::Debug for AggregationInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregationInput")
            .field("proof", &self.proof)
            .finish()
    }
}
