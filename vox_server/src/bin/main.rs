use tokio::sync::mpsc;
use vox_verse::{msg_center::msg_bus::MessageBus, server::ws_cli::run_signaling_client};

#[tokio::main]
async fn main() {
    let (bus_tx, bus_rx) = mpsc::channel(100);
    let (ws_tx, ws_rx) = mpsc::channel(100);

    tokio::spawn(async move {
        MessageBus::run(bus_rx, ws_tx).await;
    });

    let signaling_client_task = run_signaling_client(bus_tx, ws_rx);

    tokio::join!(signaling_client_task);
}
