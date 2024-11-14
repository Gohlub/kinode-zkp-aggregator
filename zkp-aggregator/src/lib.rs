use kinode_process_lib::{
    await_message, call_init, 
    http::server::{HttpServer, WsBindingConfig, HttpServerRequest, WsMessageType, send_ws_push},
    logging::{info, init_logging, Level},
    Address, Message, kiprintln, get_blob, LazyLoadBlob,
    timer::set_timer,
};
use sp1_sdk::SP1ProofWithPublicValues;
use serde::{Serialize, Deserialize};

const HTTP_SERVER_ADDRESS: &str = "http_server:distro:sys";
const TIMER_ADDRESS: &str = "timer:distro:sys";

wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v0",
});

#[derive(Serialize, Deserialize)]
pub enum TimerType {
    AggregateProofs,
}
// Request to the extension to aggregate proofs
fn setup_timer() {
    set_timer(86400000, Some(serde_json::to_vec(&TimerType::AggregateProofs).unwrap()));
}

fn handle_timer(_our: &Address,
     context: Option<Vec<u8>>,
     channel_id: &mut Option<u32>,
     http_server: &mut HttpServer)
-> anyhow::Result<()> {
    match context {
        None => Ok(()),
        Some(context) => {
            let timer_message: TimerType = serde_json::from_slice(&context)?;
            match timer_message {
                TimerType::AggregateProofs => {
                    let Some(channel_id) = channel_id else {
                        return Ok(());
                    };
                   send_ws_push(*channel_id, WsMessageType::Binary, LazyLoadBlob { mime: None, bytes: vec![] });
                    Ok(())
                }
            }
        }
    }
}

fn handle_http_server_request(our: &Address, body: &Vec<u8>, http_server: &mut HttpServer, channel_id: &mut Option<u32>) -> anyhow::Result<()> {
    let server_request: HttpServerRequest = serde_json::from_slice(body)?;
    match server_request {
        HttpServerRequest::WebSocketOpen{channel_id: ws_channel_id, ..} => {
            *channel_id = Some(ws_channel_id);
        }
        HttpServerRequest::WebSocketClose{..} => {
            *channel_id = None;
        }
        HttpServerRequest::WebSocketPush{..} => {
            let Some(blob) = get_blob() else {
                return Ok(());
            };

            let proof_with_values: SP1ProofWithPublicValues = serde_json::from_slice(blob.bytes())?;
            // ... use proof_with_values
        }
        _ => {}
    }
    
    
    
    Ok(())
}

fn handle_kinode_messages(our: &Address, source: &Address, body: &Vec<u8>) -> anyhow::Result<()> {
    Ok(())
}

fn handle_message(
    our: &Address,
    channel_id: &mut Option<u32>,
    http_server: &mut HttpServer,
) -> anyhow::Result<()> {
    let message = await_message()?;
    match message {
        Message::Response {
            source, context, ..
        } if source.process.to_string().as_str() == TIMER_ADDRESS => {
            handle_timer(our, context, channel_id, http_server)
        }
        Message::Request { source, body, .. }
            if source.process.to_string().as_str() == HTTP_SERVER_ADDRESS =>
        {
            if source.node() != our.node() {
                Ok(())
            } else {
                handle_http_server_request(our, &body, http_server, channel_id)
            }
        }
        Message::Request {
            ref source, body, ..
        } => handle_kinode_messages(our, source, &body),
        Message::Response { .. } => {
            Ok(())
        }
    }
}

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    info!("begin");
    setup_timer();

    let mut connection: Option<u32> = None;
    let mut http_server = HttpServer::new(5);
    let ws_config = WsBindingConfig::new(true, false, false, true);
    http_server.bind_ws_path("/", ws_config).unwrap();

    loop {
        match handle_message(&our, &mut connection, &mut http_server) {
            Ok(()) => {}
            Err(e) => {
                kiprintln!("error: {:?}", e);
            }
        };
    }
}
