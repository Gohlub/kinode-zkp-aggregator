use kinode_process_lib::{
    await_message, call_init, 
    http::server::{HttpServer, WsBindingConfig},
    logging::{info, init_logging, Level},
    Address, Message, kiprintln
};

const TIMER_ADDRESS: &str = "timer:distro:sys";

wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v0",
});


fn handle_request(source: &Address, body: &[u8]) -> anyhow::Result<()> {
    Ok(())
}

fn handle_message(_our: &Address, connection: &mut Option<u32>) -> anyhow::Result<()> {
    let message = await_message()?;
    match message {
        Message::Response { .. } => Ok(()),
        Message::Request { source, body, .. } => {
            handle_request(&source, &body)?;
            println!("got request from {source:?} with body {body:?}");
            Ok(())
        }
    }
}

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    info!("begin");

    let mut connection: Option<u32> = None;
    let mut http_server = HttpServer::new(5);
    let ws_config = WsBindingConfig::new(true, false, false, true);
    http_server.bind_ws_path("/", ws_config).unwrap();

    loop {
        match handle_message(&our, &mut connection) {
            Ok(()) => {}
            Err(e) => {
                kiprintln!("error: {:?}", e);
            }
        };
    }
}
