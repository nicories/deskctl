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
    OutputPowerOn { output_name: String },
    OutputPowerOff { output_name: String },
    OutputEnable { output_name: String },
    OutputDisable { output_name: String },
}
async fn autodiscover(con: &mut Connection, config: &Config, client: &AsyncClient) -> Fallible<()> {
    for output in con.get_outputs().await.unwrap() {
        {
            // dpms/power
            let cmd_on = SwayCommand::OutputPowerOn {
                output_name: output.name.clone(),
            };
            let cmd_off = SwayCommand::OutputPowerOff {
                output_name: output.name.clone(),
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
                ),
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
            // enable/disable
            let cmd_on = SwayCommand::OutputEnable {
                output_name: output.name.clone(),
            };
            let cmd_off = SwayCommand::OutputDisable {
                output_name: output.name.clone(),
            };
            let switch = config.build_switch(
                config.sway.command_topic.clone(),
                config.sway.state_topic.clone(),
                config.sway.availability.clone(),
                format!("{}_enable", output.name),
                output.name.clone(),
                output.name.clone(),
                format!(
                    "{{{{ '{on}' if (value_json.outputs[name]).dpms == true else '{off}' }}}}",
                    on = &config.switch_on_value,
                    off = &config.switch_off_value,
                ),
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
pub async fn sway_state_task(client: AsyncClient, config: Config) -> anyhow::Result<()> {
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
    log::info!("Starting sway state loop");
    while let Some(event) = events.next().await {
        state = update_state(&mut connection).await;
        client
            .publish(
                &config.sway.state_topic,
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&state)?,
            )
            .await
            .unwrap();
    }
    Ok(())
}

pub async fn sway_run() -> anyhow::Result<()> {
    log::info!("Starting sway main task");
    let config = Config::new();

    let (client, mut eventloop) = config.get_client(&config.sway);
    let (config_state, client_state) = (config.clone(), client.clone());
    // auto discover first to add the entities to home-assistant
    let mut connection = Connection::new().await?;
    autodiscover(&mut connection, &config, &client).await?;

    // then start the task to continuously update and publish the state in the background
    let _handle = task::spawn(async move {
        let result = sway_state_task(client_state, config_state).await;
        log::error!("Sway state task exited with error: {:?}", &result);
    });

    // then output the online message
    client
        .publish(
            &config.sway.availability.topic,
            QoS::AtLeastOnce,
            config.mqtt.retain_last_will,
            config.sway.availability.payload_available.clone(),
        )
        .await?;

    client
        .subscribe(&config.sway.command_topic, QoS::AtLeastOnce)
        .await?;

    // loop
    while let Ok(event) = eventloop.poll().await {
        if let rumqttc::Event::Incoming(packet) = event {
            if let rumqttc::Packet::Publish(p) = packet {
                assert_eq!(p.topic, config.sway.command_topic);
                let Ok(string) = std::str::from_utf8(&p.payload) else {
                    log::error!("Received invalid utf8 string from mqtt");
                    continue;
                };
                let Ok(sway_command) = serde_json::from_str(string) else {
                    log::error!("Could not parse json from mqtt {:?}", &string);
                    continue;
                };
                let cmd = match sway_command {
                    SwayCommand::OutputPowerOn { output_name } => {
                        format!("output {output_name} power on")
                    }
                    SwayCommand::OutputPowerOff { output_name } => {
                        format!("output {output_name} power off")
                    }
                    SwayCommand::OutputEnable { output_name } => {
                        format!("output {output_name} enable")
                    }
                    SwayCommand::OutputDisable { output_name } => {
                        format!("output {output_name} disable")
                    }
                };
                log::debug!("Running sway command: {}", &cmd);
                let Ok(output) = connection.run_command(&cmd).await else {
                    log::error!("Could not run command: {}", &cmd);
                    continue;
                };
                for result in output {
                    if result.is_err() {
                        log::error!("Error running command: {}", &cmd);
                    }
                }
            }
        }
    }
    Ok(())
}
