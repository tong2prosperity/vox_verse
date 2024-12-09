use tokio::sync::mpsc;
use vox_verse::config::CONFIG;
use vox_verse::server::ws_cli::run_signaling_client;
use vox_verse::msg_center::msg_bus::MessageBus;

#[tokio::main]
async fn main() {
    let config = CONFIG.read().await;
    let (bus_tx, bus_rx) = mpsc::channel(100);
    
    tokio::spawn(async move {
        MessageBus::run(bus_rx).await;
    });
    
    let signaling_client_task = run_signaling_client(bus_tx);

    tokio::join!(signaling_client_task);
}