use kinode_process_lib::{logging::error, set_state};
use serde::{Deserialize, Serialize};
use shared_types::AggregationInput;
use sp1_sdk::SP1ProofWithPublicValues;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub struct StateError(String);
impl std::error::Error for StateError {}
impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
pub type KinodeId = String;

#[derive(Serialize, Deserialize)]
pub enum TimerType {
    AggregateProofs,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ProofSubmissionRequest {
    pub source: KinodeId,
    pub aggregation_input: AggregationInput,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EpochState {
    proofs_by_kinode_id: HashMap<KinodeId, AggregationInput>,
    current_aggregated_proof: Option<SP1ProofWithPublicValues>,
}

impl Default for EpochState {
    fn default() -> Self {
        Self {
            proofs_by_kinode_id: HashMap::new(),
            current_aggregated_proof: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    pub current_epoch: u64,
    pub epoch_history: BTreeMap<u64, EpochState>,
}

impl Default for State {
    fn default() -> Self {
        let mut epoch_history = BTreeMap::new();
        // Initialize with epoch 0
        epoch_history.insert(0, EpochState::default());

        Self {
            current_epoch: 0,
            epoch_history,
        }
    }
}

impl State {
    pub fn save(&self) -> anyhow::Result<()> {
        match serde_json::to_vec(self) {
            Ok(serialized) => {
                set_state(&serialized);
                Ok(())
            }
            Err(e) => {
                error!("Error serializing state: {:?}", e);
                Err(anyhow::anyhow!("Error serializing state"))
            }
        }
    }

    pub fn epoch_next(&mut self) {
        // Create new EpochState for the next epoch
        let new_epoch_state = EpochState {
            proofs_by_kinode_id: HashMap::new(),
            current_aggregated_proof: None,
        };

        // Increment epoch
        self.current_epoch += 1;

        // Store the new epoch state
        self.epoch_history
            .insert(self.current_epoch, new_epoch_state);
    }

    pub fn add_proof(&mut self, kinode_id: KinodeId, proof: AggregationInput) -> bool {
        let is_new = self
            .current_epoch_state_mut()
            .map(|state| {
                state.proofs_by_kinode_id.entry(kinode_id).or_insert(proof);
                true
            })
            .unwrap_or(false);

        self.save().unwrap_or_default();
        is_new
    }

    pub fn get_proofs_for_kinode(&self, kinode_id: &str) -> Option<&AggregationInput> {
        self.current_epoch_state()?
            .proofs_by_kinode_id
            .get(kinode_id)
    }

    pub fn get_all_kinode_ids(&self) -> Vec<String> {
        self.current_epoch_state()
            .map(|state| state.proofs_by_kinode_id.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn set_aggregated_proof(&mut self, proof: SP1ProofWithPublicValues) {
        if let Some(state) = self.current_epoch_state_mut() {
            state.current_aggregated_proof = Some(proof);
            self.save().unwrap_or_default();
        }
    }

    pub fn get_aggregated_proof(&self) -> Option<&SP1ProofWithPublicValues> {
        self.current_epoch_state()?
            .current_aggregated_proof
            .as_ref()
    }

    pub fn get_epoch_state(&self, epoch: u64) -> Option<&EpochState> {
        self.epoch_history.get(&epoch)
    }

    // Helper to get current epoch state
    pub fn current_epoch_state(&self) -> Option<&EpochState> {
        self.epoch_history.get(&self.current_epoch)
    }

    // Helper to get current epoch state mutably
    pub fn current_epoch_state_mut(&mut self) -> Option<&mut EpochState> {
        self.epoch_history.get_mut(&self.current_epoch)
    }

    pub fn load(bytes: &[u8]) -> anyhow::Result<Self, StateError> {
        let old = serde_json::from_slice::<Self>(bytes);
        match old {
            Ok(s) => Ok(s),
            Err(e) => Err(StateError(e.to_string())),
        }
    }
}
