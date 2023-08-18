use swayipc::{Output, Workspace};
use tokio::{task, time};

use futures_util::stream::StreamExt;
use rumqttc::{self, AsyncClient, LastWill, MqttOptions, QoS};
use std::error::Error;
use std::time::Duration;
use swayipc_async::{Connection, EventType, Fallible};

use crate::config::Config;

struct SwayModule {}
impl SwayModule {}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SwayState {
    outputs: Vec<Output>,
    workspaces: Vec<Workspace>,
    current_workspace: String,
}

async fn update_state(con: &mut Connection) -> SwayState {
    let workspaces = con.get_workspaces().await.unwrap();
    let focused_workspace = workspaces.iter().filter(|w| w.focused).last();
    let current = match focused_workspace {
        Some(w) => w.name.clone(),
        None => "".to_owned(),
    };
    SwayState {
        outputs: con.get_outputs().await.unwrap(),
        current_workspace: current,
        workspaces: con.get_workspaces().await.unwrap(),
    }
}
async fn autodiscover(con: &mut Connection, config: &Config, client: &AsyncClient) -> Fallible<()> {
    for output in con.get_outputs().await.unwrap() {
        let switch = config.build_switch(
            config.sway.outputs_command_topic.clone(),
            config.sway.outputs_state_topic.clone(),
            config.sway.availability.clone(),
            output.name.clone(),
            output.name.clone(),
            output.name.clone(),
            config.sway.outputs_value_template.clone(),
            config.sway.outputs_attributes_template.clone(),
        );
        let topic = config.get_autodiscover_topic(&switch);
        client
            .publish(
                topic,
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&switch).unwrap(),
            )
            .await
            .unwrap();
    }
    {
        // let workspaces: Vec<String> = connection
        //     .get_workspaces()
        //     .await
        //     .unwrap()
        //     .iter()
        //     .map(|w| w.name.clone())
        //     .collect();
        // config.sway.workspaces_select.options = workspaces;
        // let topic = config.get_autodiscover_topic(&config.sway.workspaces_select);
        // client
        //     .publish(
        //         topic,
        //         QoS::AtLeastOnce,
        //         false,
        //         serde_json::to_string(&config.sway.workspaces_select).unwrap(),
        //     )
        //     .await
        //     .unwrap();
    }
    Ok(())
}

// outputs the current state of sway to the topic
pub async fn sway_state(client: AsyncClient, config: Config) -> Fallible<()> {
    let subs = [
        EventType::Workspace,
        // EventType::Output, TODO: not implemented yet in swayipc
        EventType::Window,
        // EventType::Input,
        // EventType::Tick,
        // EventType::Shutdown,
        // EventType::Mode,
        // EventType::BarStateUpdate,
        // EventType::BarConfigUpdate,
        // EventType::Binding,
    ];
    let mut connection = Connection::new().await?;

    let mut events = Connection::new().await?.subscribe(subs).await?;
    let mut state = update_state(&mut connection).await;
    while let Some(event) = events.next().await {
        if let Ok(e) = event {
            dbg!(e);
        }
        state = update_state(&mut connection).await;
        client
            .publish(
                &config.sway.state_topic,
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&state).unwrap(),
            )
            .await
            .unwrap();
        let output_state = connection.get_outputs().await.unwrap();
        client
            .publish(
                &config.sway.outputs_state_topic,
                QoS::AtLeastOnce,
                true, // retain this to avoid errors on startup
                serde_json::to_string(&output_state).unwrap(),
            )
            .await
            .unwrap();
    }
    Ok(())
}

pub async fn sway_run() -> Fallible<()> {
    let config = Config::new();

    let (client, mut eventloop) = config.get_client(&config.sway);
    let (config_state, client_state) = (config.clone(), client.clone());
    // auto discover first to add the entities to home-assistant
    let mut connection = Connection::new().await?;
    autodiscover(&mut connection, &config, &client).await?;

    // then start the task to continuously update and publish the state in the background
    task::spawn(async move {
        sway_state(client_state, config_state).await.unwrap();
    });

    // then output the online message
    client
        .publish(
            &config.sway.availability.topic,
            QoS::AtLeastOnce,
            config.mqtt.retain_last_will,
            config.sway.availability.payload_available.clone(),
        )
        .await
        .unwrap();

    // loop
    while let Ok(event) = eventloop.poll().await {
        match event {
            rumqttc::Event::Incoming(packet) => match packet {
                rumqttc::Packet::Publish(p) => {
                    dbg!(p.topic);
                }
                _ => {}
            },
            rumqttc::Event::Outgoing(_) => {}
        }
    }
    Ok(())
}
