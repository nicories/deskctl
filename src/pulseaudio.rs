use tokio::task;

use futures_util::{pin_mut, stream::StreamExt};
use rumqttc::{self, AsyncClient, QoS};

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

pub async fn pulse_state(client: AsyncClient, config: &Config) -> anyhow::Result<()> {
    log::info!("Starting pulseaudio state task");

    let pulse = Pulseaudio::new(CLIENT_NAME_STATE);
    let stream = pulse.subscribe().await;
    pin_mut!(stream);
    while let Some(s) = stream.next().await {
        if matches!(s.target, pulsectl::EventTarget::Sink) {
            log::debug!("Got pulseaudio sink event: {:?}", &s);
            let Ok(sinks) = pulse.list_sinks().await else {
                log::error!("Failed to get sinks");
                continue;
            };
            let Ok(current_sink) = pulse.get_default_sink().await else {
                log::error!("Failed to get default sink");
                continue;
            };
            let Ok(current_volume) = pulse.get_default_volume().await else {
                log::error!("Failed to get default volume");
                continue;
            };
            let state = PulseState {
                current_sink: current_sink.name,
                current_volume: current_volume.value_percent,
                sinks,
            };
            log::debug!(
                "Publishing new state {:?} to {}",
                &state,
                &config.pulseaudio.state_topic
            );
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
pub async fn pulse_run() -> anyhow::Result<()> {
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
    log::info!("Starting pulseaudio command loop");
    while let Ok(event) = eventloop.poll().await {
        if let rumqttc::Event::Incoming(rumqttc::Packet::Publish(packet)) = event {
            assert_eq!(packet.topic, config.pulseaudio.command_topic);
        }
    }

    Ok(())
}
