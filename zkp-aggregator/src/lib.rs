use dotenvy::from_read;
use kinode_process_lib::{
    await_message, call_init, get_blob, get_typed_state,
    http::server::{send_ws_push, HttpServer, HttpServerRequest, WsBindingConfig, WsMessageType},
    kiprintln,
    logging::{info, init_logging, Level},
    timer::set_timer,
    Address, LazyLoadBlob, Message,
};
use lazy_static::lazy_static;
use shared_types::{AggregationInput, AggregationOutput, WsMessage};
use std::env;
use std::io::Cursor;
pub mod caller;
pub mod contract_caller;
pub mod structs;
use caller::Caller;
use contract_caller::ContractCaller;
use structs::*;
lazy_static! {
    pub static ref CURRENT_CHAIN_ID: u64 = {
        let env_content = include_str!("../../.env");
        from_read(Cursor::new(env_content)).expect("Failed to parse .env content");
        env::var("CURRENT_CHAIN_ID")
            .expect("CHAIN_ID must be set")
            .parse()
            .unwrap()
    };
    pub static ref SP1_AGGREGATE_VERIFIER_CONTRACT_ADDRESS: String = {
        let env_content = include_str!("../../.env");
        from_read(Cursor::new(env_content)).expect("Failed to parse .env content");
        env::var("SP1_AGGREGATE_VERIFIER_CONTRACT_ADDRESS")
            .expect("SP1_AGGREGATE_VERIFIER_CONTRACT_ADDRESS must be set")
    };
    pub static ref WALLET_PRIVATE_KEY: String = {
        let env_content = include_str!("../../.env");
        from_read(Cursor::new(env_content)).expect("Failed to parse .env content");
        env::var("WALLET_PRIVATE_KEY").expect("WALLET_PRIVATE_KEY must be set")
    };
    pub static ref RPC_URL: String = {
        let env_content = include_str!("../../.env");
        from_read(Cursor::new(env_content)).expect("Failed to parse .env content");
        env::var("CURRENT_RPC_URL").expect("RPC_URL must be set")
    };
}

const HTTP_SERVER_ADDRESS: &str = "http_server:distro:sys";
const TIMER_ADDRESS: &str = "timer:distro:sys";
wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v0",
});

// Request to the extension to aggregate proofs, set to 5 mins
fn setup_timer() {
    set_timer(
        300000,
        Some(serde_json::to_vec(&TimerType::AggregateProofs).unwrap()),
    );
}

fn handle_timer(
    _our: &Address,
    context: Option<Vec<u8>>,
    channel_id: &mut Option<u32>,
    _http_server: &mut HttpServer,
    state: &mut State,
) -> anyhow::Result<()> {
    match context {
        None => Ok(()),
        Some(context) => {
            let timer_message: TimerType = serde_json::from_slice(&context)?;
            match timer_message {
                TimerType::AggregateProofs => {
                    let Some(channel_id) = channel_id else {
                        kiprintln!("No channel id");
                        return Ok(());
                    };
                    // Send aggregate proofs from state
                    let proofs: Vec<AggregationInput> = state
                        .get_all_kinode_ids()
                        .iter()
                        .filter_map(|kinode_id| state.get_proofs_for_kinode(kinode_id))
                        .cloned()
                        .collect();
                    let serialized_proofs = serde_json::to_vec(&proofs)?;
                    send_ws_push(
                        *channel_id,
                        WsMessageType::Binary,
                        LazyLoadBlob {
                            mime: None,
                            bytes: serialized_proofs,
                        },
                    );

                    // Go to the next epoch in the state
                    state.epoch_next();
                    Ok(())
                }
            }
        }
    }
}
// From the terminal
fn send_to_chain(
    output: AggregationOutput,
    eth_caller: &mut Option<ContractCaller>,
) -> anyhow::Result<()> {
    if let Some(caller) = eth_caller.as_ref() {
        caller.verify_aggregate_proof_and_update_root(output)?;
        Ok(())
    } else {
        Err(anyhow::anyhow!("eth_caller is None"))
    }
}

fn handle_http_server_request(
    _our: &Address,
    body: &Vec<u8>,
    _http_server: &mut HttpServer,
    channel_id: &mut Option<u32>,
    state: &mut State,
    _eth_caller: &mut Option<ContractCaller>,
) -> anyhow::Result<()> {
    let server_request: HttpServerRequest = serde_json::from_slice(body)?;
    match server_request {
        HttpServerRequest::WebSocketOpen {
            channel_id: ws_channel_id,
            ..
        } => {
            kiprintln!("WebSocket opened");
            *channel_id = Some(ws_channel_id);
        }
        HttpServerRequest::WebSocketClose { .. } => {
            *channel_id = None;
        }
        // Should probably have a type for this
        HttpServerRequest::WebSocketPush { .. } => {
            let blob = match get_blob() {
                Some(b) => b,
                None => {
                    kiprintln!("No blob received");
                    return Ok(());
                }
            };

            let raw_message = String::from_utf8_lossy(blob.bytes());

            match serde_json::from_slice::<WsMessage>(blob.bytes()) {
                Ok(WsMessage::Aggregation(output)) => {
                    // send_to_chain(output.clone(), eth_caller)?;
                    kiprintln!("Sent to chain");
                    kiprintln!("Setting aggregated proof: {:?}", output.proof);
                    state.set_aggregated_proof(output.proof.clone());
                    state.epoch_next();
                }
                // Had to insert the dummy votes into the state on the WS client side
                // since I couldn't get proof objects to load from the vfs
                Ok(WsMessage::DummyProof(dummy)) => {
                    kiprintln!("Received dummy proof insert message");
                    // Initialize the current epoch if it doesn't exist
                    if !state.epoch_history.contains_key(&state.current_epoch) {
                        state
                            .epoch_history
                            .insert(state.current_epoch, Default::default());
                    }
                    // Add each proof to the current epoch
                    for proof in dummy.proofs {
                        state.add_proof("fake.dev".to_string(), proof);
                    }
                }
                Err(e) => {
                    kiprintln!(
                        "Invalid message format: {:?}. Raw message: {}",
                        e,
                        raw_message
                    );
                }
            }
        }
        _ => {}
    }
    Ok(())
}
// Had to insert the dummy votes into the state on the WS client side
// since I couldn't get proof objects to load from the vfs
fn handle_insert_dummy_proofs(
    _state: &mut State,
    _our: &Address,
    channel_id: &Option<u32>,
) -> anyhow::Result<()> {
    let Some(channel) = channel_id else {
        return Ok(());
    };
    let message: String = "insert_proofs_pls".to_string();
    send_ws_push(
        *channel,
        WsMessageType::Text,
        LazyLoadBlob {
            mime: None,
            bytes: message.as_bytes().to_vec(),
        },
    );
    kiprintln!("Sent dummy proof insert message");
    Ok(())
}

fn handle_kinode_messages(
    our: &Address,
    source: &Address,
    body: &Vec<u8>,
    state: &mut State,
    channel_id: &mut Option<u32>,
    eth_caller: &mut Option<ContractCaller>,
) -> anyhow::Result<()> {
    if source.package() == "terminal" {
        return handle_terminal_debug(&body, state, our, channel_id, eth_caller);
    }
    let request_type: Result<ProofSubmissionRequest, _> = serde_json::from_slice(body);
    if let Ok(proof_submission_request) = request_type {
        match proof_submission_request {
            ProofSubmissionRequest {
                source,
                aggregation_input,
            } => {
                state.add_proof(source, aggregation_input);
                Ok(())
            }
        }
    } else {
        Err(anyhow::anyhow!(
            "Request type does not match ProofSubmissionRequest"
        ))
    }
}
fn handle_terminal_debug(
    body: &Vec<u8>,
    state: &mut State,
    our: &Address,
    channel_id: &mut Option<u32>,
    eth_caller: &mut Option<ContractCaller>,
) -> anyhow::Result<()> {
    let body = String::from_utf8(body.to_vec())?;
    let command = body.as_str();
    match command {
        "print_state" => {
            kiprintln!("Printing state");
            kiprintln!("State: {:?}", state);
        }
        "current_epoch" => {
            kiprintln!("Current Epoch: {:?}", state.current_epoch);
            kiprintln!(
                "Epoch State: {:?}",
                state.epoch_history.get(&state.current_epoch)
            );
        }
        "list_epochs" => {
            kiprintln!(
                "Epochs: {:?}",
                state.epoch_history.keys().collect::<Vec<&u64>>()
            );
        }
        cmd if cmd.starts_with("print_epoch:") => {
            if let Some(epoch_str) = cmd.split(':').nth(1) {
                if let Ok(epoch) = epoch_str.parse::<u64>() {
                    if let Some(epoch_state) = state.epoch_history.get(&epoch) {
                        kiprintln!("Epoch state for epoch {}: {:?}", epoch, epoch_state);
                    } else {
                        kiprintln!("No epoch state found for epoch {}", epoch);
                    }
                }
            }
        }
        "insert_dummy_proofs" => {
            handle_insert_dummy_proofs(state, our, channel_id)?;
        }
        "request_aggregate_proofs" => {
            let Some(channel_id) = channel_id else {
                kiprintln!("No channel id");
                return Ok(());
            };
            // Send aggregate proofs from state
            let proofs: Vec<AggregationInput> = state
                .get_all_kinode_ids()
                .iter()
                .filter_map(|kinode_id| state.get_proofs_for_kinode(kinode_id))
                .cloned()
                .collect();
            let serialized_proofs = serde_json::to_vec(&proofs)?;
            send_ws_push(
                *channel_id,
                WsMessageType::Binary,
                LazyLoadBlob {
                    mime: None,
                    bytes: serialized_proofs,
                },
            );
        }
        "send_to_chain" => {
            let Some(proof) = state.get_aggregated_proof() else {
                kiprintln!("No aggregated proof");
                return Ok(());
            };
            let output = AggregationOutput {
                proof: proof.clone(),
            };
            send_to_chain(output, eth_caller)?;
        }
        _ => {
            kiprintln!("Unknown command: {}", command);
        }
    }

    Ok(())
}

fn handle_message(
    our: &Address,
    channel_id: &mut Option<u32>,
    http_server: &mut HttpServer,
    state: &mut State,
    eth_caller: &mut Option<ContractCaller>,
) -> anyhow::Result<()> {
    let message = await_message()?;
    match message {
        Message::Response {
            source, context, ..
        } if source.process.to_string().as_str() == TIMER_ADDRESS => {
            handle_timer(our, context, channel_id, http_server, state)
        }
        Message::Request { source, body, .. } => match source.process.to_string().as_str() {
            HTTP_SERVER_ADDRESS => {
                if source.node() != our.node() {
                    kiprintln!("I am tring to snoop, my name is: {:?}", source.node());
                    Ok(())
                } else {
                    handle_http_server_request(
                        our,
                        &body,
                        http_server,
                        channel_id,
                        state,
                        eth_caller,
                    )
                }
            }
            _ => handle_kinode_messages(our, &source, &body, state, channel_id, eth_caller),
        },
        Message::Response { .. } => Ok(()),
    }
}

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    kiprintln!("Initializing zkp-aggregator");
    info!("begin");
    setup_timer();

    let mut state: State = get_typed_state(|bytes| State::load(bytes)).unwrap_or_default();

    let mut eth_caller: Option<ContractCaller> = Some(ContractCaller {
        caller: Caller::new(*CURRENT_CHAIN_ID, &WALLET_PRIVATE_KEY).unwrap(),
        contract_address: SP1_AGGREGATE_VERIFIER_CONTRACT_ADDRESS.to_string(),
    });

    let mut channel_id: Option<u32> = None;
    let mut http_server = HttpServer::new(5);
    let ws_config = WsBindingConfig::new(false, false, false, true);
    http_server.bind_ws_path("/", ws_config).unwrap();

    loop {
        match handle_message(
            &our,
            &mut channel_id,
            &mut http_server,
            &mut state,
            &mut eth_caller,
        ) {
            Ok(()) => {}
            Err(e) => {
                kiprintln!("error: {:?}", e);
            }
        };
    }
}
