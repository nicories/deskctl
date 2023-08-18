use log::log_enabled;
use tokio::{join, task, time};

use futures_util::stream::StreamExt;
use rumqttc::{self, AsyncClient, MqttOptions, QoS};
use std::error::Error;
use std::time::Duration;
use swayipc_async::{Connection, EventType, Fallible};

mod config;
mod homeassistant;
mod pulseaudio;
mod sway;

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
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
        sway::sway_run().await.unwrap();
    });
    let pulse_handle = task::spawn(async move {
        pulseaudio::pulse_run().await.unwrap();
    });
    pulse_handle.await.unwrap();
    sway_handle.await.unwrap();

    Ok(())
}
