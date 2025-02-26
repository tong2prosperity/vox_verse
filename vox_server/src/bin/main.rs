use std::io::Write;
use std::str::FromStr;
use tokio::sync::mpsc;
use vox_verse::{debug, error, info, warn};
use vox_verse::{msg_center::msg_bus::MessageBus, server::ws_cli::run_signaling_client};
#[tokio::main]
async fn main() {
    // env_logger::Builder::new()
    //     .format(|buf, record| {
    //         writeln!(
    //             buf,
    //             "{}:{} [{}] {} - {}",
    //             record.file().unwrap_or("unknown"),
    //             record.line().unwrap_or(0),
    //             record.level(),
    //             chrono::Local::now().format("%H:%M:%S.%6f"),
    //             record.args()
    //         )
    //     })
    //     .filter(None, log::LevelFilter::Trace)
    //     .init();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();
    info!("Starting vox_server...");
    let (bus_tx, bus_rx) = mpsc::channel(100);
    let (ws_tx, ws_rx) = mpsc::channel(100);

    info!("Spawning MessageBus task");
    tokio::spawn(async move {
        info!("MessageBus task started");
        MessageBus::run(bus_rx, ws_tx).await;
        warn!("MessageBus task exited");
    });

    info!("Starting signaling client");
    let signaling_client_task = run_signaling_client(bus_tx, ws_rx);

    info!("Waiting for tasks to complete");
    tokio::join!(signaling_client_task);
    info!("Server shutting down");
}
