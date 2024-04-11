use log::log_enabled;
use tokio::task;

mod config;
mod homeassistant;
mod pulseaudio;
mod sway;

#[tokio::main(worker_threads = 1)]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    log::info!("Starting desktop");
    if log_enabled!(log::Level::Debug) {
        log::info!("Debug logging enabled");
    }
    if log_enabled!(log::Level::Error) {
        log::info!("Error logging enabled");
    }
    let sway_handle = task::spawn(async move {
        sway::sway_run().await.expect("sway_run");
    });
    let pulse_handle = task::spawn(async move {
        pulseaudio::pulse_run().await.unwrap();
    });
    pulse_handle.await.unwrap();
    sway_handle.await.unwrap();

    Ok(())
}
