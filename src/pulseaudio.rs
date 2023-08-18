use swayipc::{Output, Workspace};
use tokio::{task, time};

use futures_util::{pin_mut, stream::StreamExt};
use rumqttc::{self, AsyncClient, LastWill, MqttOptions, QoS};
use std::error::Error;
use std::time::Duration;
use swayipc_async::{Connection, EventType, Fallible};

use pulsectl::Pulseaudio;

const CLIENT_NAME_CMD: &str = "desktop-cmd";
const CLIENT_NAME_STATE: &str = "desktop-state";

use crate::config::Config;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct PulseState {
    sinks: Vec<pulsectl::SinkInfo>,
    current_sink: String,
    current_volume: String,
}

pub async fn pulse_state(client: AsyncClient, config: &Config) -> Fallible<()> {
    log::info!("Starting pulseaudio state task");

    let pulse = Pulseaudio::new(CLIENT_NAME_STATE);
    let stream = pulse.subscribe().await;
    pin_mut!(stream);
    while let Some(s) = stream.next().await {
        if s.on == "sink" {
            log::debug!("Got pulseaudio sink event: {:?}", &s);
            let sinks = pulse.list_sinks().await.unwrap();
            let current_sink = pulse.get_default_sink().await.unwrap().name;
            let current_volume = pulse.get_default_volume().await.unwrap().value_percent;
            let state = PulseState {
                current_sink,
                current_volume,
                sinks,
            };
            log::debug!("Publishing new state to {}", &config.pulseaudio.state_topic);
            client
                .publish(
                    &config.pulseaudio.state_topic,
                    QoS::AtLeastOnce,
                    false,
                    serde_json::to_string(&state).unwrap(),
                )
                .await
                .unwrap();
            log::debug!("Published new state");
        }
    }
    Ok(())
}
pub async fn pulse_run() -> Fallible<()> {
    log::info!("Starting pulseaudio main task");
    let config = Config::new();
    let pulse = Pulseaudio::new(CLIENT_NAME_CMD);

    let (client, mut eventloop) = config.get_client(&config.pulseaudio);
    let (config_state, client_state) = (config.clone(), client.clone());
    // autodiscover(&mut connection, &config, &client).await?;

    // // then start the task to continuously update and publish the state in the background
    task::spawn(async move {
        pulse_state(client_state, &config_state).await.unwrap();
    });
    while let Ok(event) = eventloop.poll().await {
        match event {
            rumqttc::Event::Incoming(packet) => match packet {
                rumqttc::Packet::Publish(p) => {
                    // dbg!(p.topic);
                }
                _ => {}
            },
            rumqttc::Event::Outgoing(_) => {}
        }
    }

    Ok(())
}
