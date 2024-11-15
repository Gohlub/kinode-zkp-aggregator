use kinode_process_lib::{
    await_message, call_init, 
    http::server::{HttpServer, WsBindingConfig, HttpServerRequest, WsMessageType, send_ws_push},
    logging::{info, init_logging, Level},
    Address, Message, kiprintln, get_blob, LazyLoadBlob,
    timer::set_timer, get_typed_state,
};
use sp1_sdk::SP1ProofWithPublicValues;
pub mod structs;
use structs::*;

const HTTP_SERVER_ADDRESS: &str = "http_server:distro:sys";
const TIMER_ADDRESS: &str = "timer:distro:sys";
wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v0",
});

// Request to the extension to aggregate proofs
fn setup_timer() {
    set_timer(86400000, Some(serde_json::to_vec(&TimerType::AggregateProofs).unwrap()));
}

fn handle_timer(_our: &Address,
     context: Option<Vec<u8>>,
     channel_id: &mut Option<u32>,
     _http_server: &mut HttpServer,
     state: &mut State)
-> anyhow::Result<()> {
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
                    let proofs: Vec<AggregationInput> = state.get_all_kinode_ids()
                        .iter()
                        .filter_map(|kinode_id| state.get_proofs_for_kinode(kinode_id))
                        .cloned()
                        .collect();
                    let serialized_proofs = serde_json::to_vec(&proofs)?;
                    send_ws_push(*channel_id, WsMessageType::Binary, LazyLoadBlob { mime: None, bytes: serialized_proofs });
                    
                    // Go to the next epoch in the state
                    state.epoch_next();
                    
                    Ok(())
                }
            }
        }
    }
}

fn handle_http_server_request(_our: &Address,
     body: &Vec<u8>,
     _http_server: &mut HttpServer,
     channel_id: &mut Option<u32>,
     state: &mut State) -> anyhow::Result<()> {
    let server_request: HttpServerRequest = serde_json::from_slice(body)?;
    match server_request {
        HttpServerRequest::WebSocketOpen{channel_id: ws_channel_id, ..} => {
            kiprintln!("WebSocket opened");
            *channel_id = Some(ws_channel_id);
        }
        HttpServerRequest::WebSocketClose{..} => {
            *channel_id = None;
        }
        HttpServerRequest::WebSocketPush{..} => {
            let Some(blob) = get_blob() else {
                return Ok(());
            };
            let aggregated_proof: SP1ProofWithPublicValues = serde_json::from_slice(blob.bytes())?;
            state.set_aggregated_proof(aggregated_proof);
        }
        _ => {}
    }
    Ok(())
}

fn handle_kinode_messages(_our: &Address, source: &Address, body: &Vec<u8>, state: &mut State) -> anyhow::Result<()> {
    if source.package() == "terminal" {
        return handle_terminal_debug(&body, state);
    }
    let proof_submission_request: ProofSubmissionRequest = serde_json::from_slice(body)?;
    match proof_submission_request {
        ProofSubmissionRequest::AggregationInput(aggregation_input) => {
            state.add_proof(source.to_string(), aggregation_input);
            Ok(())
        }
    }
}
fn handle_terminal_debug(body: &Vec<u8>, state: &mut State) -> anyhow::Result<()> {
    let body = String::from_utf8(body.to_vec())?;
    let command = body.as_str();
    match command {
        "print_state" => {
            kiprintln!("Printing state");
            kiprintln!("State: {:?}", state);
        }
        "current_epoch" => {
            kiprintln!("Current Epoch: {:?}", state.current_epoch);
            kiprintln!("Epoch State: {:?}", state.epoch_history.get(&state.current_epoch));
        }
        "list_epochs" => {
            kiprintln!("Epochs: {:?}", state.epoch_history.keys().collect::<Vec<&u64>>());
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
) -> anyhow::Result<()> {
    let message = await_message()?;
    match message {
        Message::Response {
            source, context, ..
        } if source.process.to_string().as_str() == TIMER_ADDRESS => {
            handle_timer(our, context, channel_id, http_server, state)
        }
        Message::Request { source, body, .. } => {
            match source.process.to_string().as_str() {
                HTTP_SERVER_ADDRESS => {
                    if source.node() != our.node() {
                        kiprintln!("I am tring to snoop, my name is: {:?}", source.node());
                        Ok(())
                    } else {
                        handle_http_server_request(our, &body, http_server, channel_id, state)
                    }
                }
                _ => handle_kinode_messages(our, &source, &body, state),
            }
        }
        Message::Response { .. } => {
            Ok(())
        }
    }
}

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    kiprintln!("Initializing zkp-aggregator");
    info!("begin");
    setup_timer();

    let mut state: State = get_typed_state(|bytes| State::load(bytes)).unwrap_or_default();

    let mut connection: Option<u32> = None;
    let mut http_server = HttpServer::new(5);
    let ws_config = WsBindingConfig::new(false, false, false, true);
    http_server.bind_ws_path("/", ws_config).unwrap();

    loop {
        match handle_message(&our, &mut connection, &mut http_server, &mut state) {
            Ok(()) => {}
            Err(e) => {
                kiprintln!("error: {:?}", e);
            }
        };
    }
}
