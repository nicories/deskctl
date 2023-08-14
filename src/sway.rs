use swayipc::{Output, Workspace};
use tokio::{task, time};

use futures_util::stream::StreamExt;
use rumqttc::{self, AsyncClient, LastWill, MqttOptions, QoS};
use std::error::Error;
use std::time::Duration;
use swayipc_async::{Connection, EventType, Fallible};

use crate::config::{ComponentSelect, ComponentSwitch, Config};

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

// outputs the current state of sway to the topic
pub async fn sway_state(client: AsyncClient, mut config: Config) -> Fallible<()> {
    let subs = [
        EventType::Workspace,
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
    // auto discover
    for output in connection.get_outputs().await.unwrap() {
        // let select = ComponentSwitch { common: todo!() };
        let switch = config.build_switch(
            config.sway.outputs_command_topic.clone(),
            config.sway.outputs_state_topic.clone(),
            config.sway.availability.clone(),
            output.name.clone(),
            output.name.clone(),
            output.name.clone(),
            "{{ 'ON' if this.attributes.dpms else 'OFF' }}".to_owned(),
            "{{ value_json | selectattr('name', 'equalto', name) | first | tojson }}".to_owned(),
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

    let mut events = Connection::new().await?.subscribe(subs).await?;
    let mut state = update_state(&mut connection).await;
    while let Some(event) = events.next().await {
        state = update_state(&mut connection).await;
        let output_state = connection.get_outputs().await.unwrap();
        client
            .publish(
                &config.sway.state_topic,
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&state).unwrap(),
            )
            .await
            .unwrap();
        client
            .publish(
                &config.sway.outputs_state_topic,
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&output_state).unwrap(),
            )
            .await
            .unwrap();
        println!("{:?}\n", event?)
    }
    Ok(())
}

pub async fn sway_run() -> Fallible<()> {
    let mut config = Config::new();

    let (client, mut eventloop) = config.get_client(&config.sway.availability.topic);
    let config_copy = config.clone();
    // online message
    client
        .publish(
            &config.sway.availability.topic,
            QoS::AtLeastOnce,
            config.mqtt.retain_last_will,
            config.sway.availability.payload_available.clone(),
        )
        .await
        .unwrap();

    task::spawn(async move {
        sway_state(client, config_copy).await.unwrap();
    });
    let mut connection = Connection::new().await?;

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
