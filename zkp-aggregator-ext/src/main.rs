use dotenv::dotenv;
use futures_util::{SinkExt, StreamExt};
use shared_types::{AggregationInput, AggregationOutput, DummyProofInsert};
use sp1_sdk::network::proto::network::ProofMode;
use sp1_sdk::{
    include_elf, HashableKey, NetworkProverV1, SP1Proof, SP1ProofWithPublicValues, SP1Stdin,
    SP1VerifyingKey,
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

pub const AGGREGATOR_ELF: &[u8] = include_elf!("aggregator-program");

async fn handle_insert_dummy_proofs() -> anyhow::Result<Vec<AggregationInput>> {
    let mut proofs: Vec<AggregationInput> = Vec::new();
    let proof_paths = vec![
        "./dummy-proofs/proof-with-pis-0.bin",
        "./dummy-proofs/proof-with-pis-1.bin",
        "./dummy-proofs/proof-with-pis-2.bin",
    ];
    let vk_paths = vec![
        "./dummy-proofs/verifying-key-0.json",
        "./dummy-proofs/verifying-key-1.json",
        "./dummy-proofs/verifying-key-2.json",
    ];

    for (proof_path, vk_path) in proof_paths.iter().zip(vk_paths.iter()) {
        let proof = SP1ProofWithPublicValues::load(proof_path)
            .map_err(|e| anyhow::anyhow!("Failed to load proof from {}: {}", proof_path, e))?;
        let vk_file = std::fs::File::open(vk_path)
            .map_err(|e| anyhow::anyhow!("Failed to open verifying key file {}: {}", vk_path, e))?;
        let vk: SP1VerifyingKey = serde_json::from_reader(vk_file).map_err(|e| {
            anyhow::anyhow!(
                "Failed to deserialize verifying key from {}: {}",
                vk_path,
                e
            )
        })?;
        proofs.push(AggregationInput { proof, vk });
    }
    Ok(proofs)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let url =
        Url::parse("ws://localhost:8080/zkp-aggregator:zkp-aggregator:punctumfix.os").unwrap();
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
            Ok(Message::Text(request)) => match request.as_str() {
                "insert_proofs_pls" => {
                    let proofs = handle_insert_dummy_proofs().await?;
                    let dummy_insert = DummyProofInsert { proofs };
                    let serialized_proofs = serde_json::to_vec(&dummy_insert)?;
                    write.send(Message::Binary(serialized_proofs)).await?;
                }
                "send" => {
                    write.send(Message::Text("pong".to_string())).await?;
                }
                _ => {}
            },
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
    println!("Proving...");
    let proof = network_prover
        .prove(AGGREGATOR_ELF, aggregate_stdin, ProofMode::Groth16, None)
        .await
        .map_err(|e| anyhow::anyhow!("Proving failed: {}", e))?;

    serde_json::to_vec(&AggregationOutput { proof })
        .map_err(|e| anyhow::anyhow!("Serialization failed: {}", e))
}
