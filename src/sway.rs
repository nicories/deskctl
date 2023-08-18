use swayipc::{Output, Workspace};
use tokio::{task, time};

use futures_util::stream::StreamExt;
use rumqttc::{self, AsyncClient, LastWill, MqttOptions, QoS};
use std::time::Duration;
use std::{collections::HashMap, error::Error};
use swayipc_async::{Connection, EventType, Fallible};

use crate::config::Config;

struct SwayModule {}
impl SwayModule {}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct SwayState {
    outputs: HashMap<String, Output>,
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
    let outputs = con.get_outputs().await.unwrap();
    let map: HashMap<String, Output> = outputs
        .iter()
        .map(|o| (o.name.clone(), o.clone()))
        .collect();
    SwayState {
        outputs: map,
        current_workspace: current,
        workspaces: con.get_workspaces().await.unwrap(),
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag = "type")]
enum SwayCommand {
    Dpms { output_name: String, set: bool },
}
async fn autodiscover(con: &mut Connection, config: &Config, client: &AsyncClient) -> Fallible<()> {
    for output in con.get_outputs().await.unwrap() {
        let cmd_on = SwayCommand::Dpms {
            output_name: output.name.clone(),
            set: true,
        };
        let cmd_off = SwayCommand::Dpms {
            output_name: output.name.clone(),
            set: false,
        };
        let switch = config.build_switch(
            config.sway.command_topic.clone(),
            config.sway.state_topic.clone(),
            config.sway.availability.clone(),
            output.name.clone(),
            output.name.clone(),
            output.name.clone(),
            format!(
                "{{{{ '{on}' if (value_json.outputs[name]).dpms == true else '{off}' }}}}",
                on = &config.switch_on_value,
                off = &config.switch_off_value,
            )
            .to_owned(),
            format!("{{{{ value_json.outputs[name] | tojson }}}}"),
            serde_json::to_string(&cmd_on).unwrap(),
            serde_json::to_string(&cmd_off).unwrap(),
        );
        let topic = config.get_autodiscover_topic(&switch);
        client
            .publish(
                topic,
                QoS::AtLeastOnce,
                true,
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
    log::info!("Starting sway state task");
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
    }
    Ok(())
}

pub async fn sway_run() -> Fallible<()> {
    log::info!("Starting sway main task");
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

    client
        .subscribe(&config.sway.command_topic, QoS::AtLeastOnce)
        .await
        .unwrap();

    // loop
    while let Ok(event) = eventloop.poll().await {
        if let rumqttc::Event::Incoming(packet) = event {
            if let rumqttc::Packet::Publish(p) = packet {
                assert_eq!(p.topic, config.pulseaudio.command_topic);
                let cmd: SwayCommand =
                    serde_json::from_str(std::str::from_utf8(&p.payload).unwrap()).unwrap();
                match cmd {
                    SwayCommand::Dpms { output_name, set } => {
                        let state = if set { "on" } else { "off" };
                        connection
                            .run_command(format!("output {output_name} dpms {state}"))
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }
    Ok(())
}
