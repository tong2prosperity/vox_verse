use tokio::sync::mpsc;
use vox_verse::{msg_center::msg_bus::MessageBus, server::ws_cli::run_signaling_client};
use vox_verse::{info, debug, error, warn};
#[tokio::main]
async fn main() {
    info!("Starting vox_server...");
    let (bus_tx, bus_rx) = mpsc::channel(100);
    let (ws_tx, ws_rx) = mpsc::channel(100);

    info!("Spawning MessageBus task");
    tokio::spawn(async move {
        info!("MessageBus task started");
        MessageBus::run(bus_rx, ws_tx).await;
    });

    info!("Starting signaling client");
    let signaling_client_task = run_signaling_client(bus_tx);

    info!("Waiting for tasks to complete");
    tokio::join!(signaling_client_task);
    info!("Server shutting down");
}
