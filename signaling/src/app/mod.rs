use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub sender: broadcast::Sender<String>,
}
