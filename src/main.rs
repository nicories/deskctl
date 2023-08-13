use tokio::{join, task, time};

use futures_util::stream::StreamExt;
use rumqttc::{self, AsyncClient, MqttOptions, QoS};
use std::error::Error;
use std::time::Duration;
use swayipc_async::{Connection, EventType, Fallible};

mod config;
mod sway;

#[tokio::main(worker_threads = 1)]
async fn main() -> Result<(), Box<dyn Error>> {
    let sway_handle = task::spawn(async move {
        sway::sway_run().await.unwrap();
    });
    sway_handle.await.unwrap();

    Ok(())
}
