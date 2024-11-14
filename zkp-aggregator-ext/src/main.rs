use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use url::Url;
use serde::{Serialize, Deserialize};
use sp1_sdk::{
    include_elf, SP1ProofWithPublicValues, SP1Stdin,
    SP1VerifyingKey, NetworkProverV1, SP1Proof, HashableKey, 
};
use sp1_sdk::network::proto::network::ProofMode;
use dotenv::dotenv;

pub const AGGREGATOR_ELF: &[u8] = include_elf!("aggregator-program");
#[derive(Serialize, Deserialize)]
struct AggregationInput {
    proof: SP1ProofWithPublicValues,
    vk: SP1VerifyingKey,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    
    let url = Url::parse("ws://localhost:8080/zkp-aggregator:zkp-aggregator:punctumfix.os").unwrap();
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();
    
    println!("WebSocket connected!");

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Binary(request)) => {
                let batch: Vec<AggregationInput> = serde_json::from_slice(&request)?;
                let proof = process_aggregation(batch).await?;
                write.send(Message::Binary(proof)).await?;
            }
            Ok(Message::Close(_)) => {
                eprintln!("Server closed the connection");
                return Err(anyhow::anyhow!("Server closed the connection"));
            }
            Err(e) => {
                eprintln!("Error in receiving message: {}", e);
                return Err(anyhow::anyhow!("Error in receiving message: {}", e));
            }
            _ => {}
        }
    }

    Ok(())
}

async fn process_aggregation(batch: Vec<AggregationInput>) -> anyhow::Result<Vec<u8>> {
    let network_prover = NetworkProverV1::new();
    let mut aggregate_stdin = SP1Stdin::new();

    let vks: Vec<_> = batch.iter().map(|input| input.vk.hash_u32()).collect();
    aggregate_stdin.write(&vks);

    let pub_vals: Vec<_> = batch
        .iter()
        .map(|input| input.proof.public_values.to_vec())
        .collect();
    aggregate_stdin.write(&pub_vals);

    for input in batch {
        let SP1Proof::Compressed(proof) = input.proof.proof else {
            panic!()
        };
        aggregate_stdin.write_proof(*proof, input.vk.vk);
    }    

    let proof = network_prover.prove(AGGREGATOR_ELF, aggregate_stdin, ProofMode::Groth16, None)
        .await
        .map_err(|e| anyhow::anyhow!("Proving failed: {}", e))?;

    serde_json::to_vec(&proof)
        .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))
}

