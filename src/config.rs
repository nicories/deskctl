use std::time::Duration;

use rumqttc::AsyncClient;
use rumqttc::EventLoop;
use rumqttc::LastWill;
use rumqttc::MqttOptions;
use rumqttc::QoS;
use serde::Deserialize;
use serde::Serialize;

use crate::homeassistant::Availability;
use crate::homeassistant::Component;
use crate::homeassistant::ComponentCommon;
use crate::homeassistant::Device;
use crate::homeassistant::Select;
use crate::homeassistant::Switch;

static CONFIG_STR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/default_config.yaml"
));

pub trait MqttModuleConfig {
    fn client_id(&self) -> &str;
    fn last_will_topic(&self) -> &str;
    fn last_will_payload(&self) -> String;
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MqttConfig {
    pub server_host: String,
    pub server_port: u16,
    pub keep_alive: u64,
    pub user: Option<String>,
    pub password: Option<String>,
    pub retain_last_will: bool,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct HomeAssistantConfig {
    pub autodiscover: bool,
    /// prefix used in homeassistant discovery, see https://www.home-assistant.io/integrations/mqtt/#discovery-options
    pub autodiscover_prefix: String,
    pub device: Device,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptConfig {
    command: String,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PulseAudioConfig {
    pub mqtt_name: String,
    pub state_topic: String,
    pub command_topic: String,
    pub availability: Availability,
}
impl MqttModuleConfig for PulseAudioConfig {
    fn client_id(&self) -> &str {
        &self.mqtt_name
    }

    fn last_will_topic(&self) -> &str {
        &self.availability.topic
    }

    fn last_will_payload(&self) -> String {
        self.availability.payload_not_available.clone()
    }
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SwayConfig {
    pub name_prefix: String,
    pub mqtt_name: String,
    pub state_topic: String,
    pub command_topic: String,
    pub availability: Availability,
}
impl MqttModuleConfig for SwayConfig {
    fn client_id(&self) -> &str {
        &self.mqtt_name
    }

    fn last_will_topic(&self) -> &str {
        &self.availability.topic
    }

    fn last_will_payload(&self) -> String {
        self.availability.payload_not_available.clone()
    }
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub app_name: String,
    pub mqtt: MqttConfig,
    pub homeassistant: HomeAssistantConfig,
    pub pulseaudio: PulseAudioConfig,
    pub sway: SwayConfig,
    pub switch_on_value: String,
    pub switch_off_value: String,
    // scripts: Vec<ScriptConfig>,
}

impl Config {
    pub fn new() -> Self {
        // construct config and add it to the template environment
        let config: Config = serde_yaml::from_str(CONFIG_STR).expect("default config");
        config
    }

    pub fn get_client(&self, mqtt_config: &dyn MqttModuleConfig) -> (AsyncClient, EventLoop) {
        log::debug!(
            "Connecting to mqtt broker at {}:{} with client_id: {}",
            self.mqtt.server_host,
            self.mqtt.server_port,
            mqtt_config.client_id()
        );

        let mut mqttoptions = MqttOptions::new(
            mqtt_config.client_id(),
            &self.mqtt.server_host,
            self.mqtt.server_port,
        )
        .set_keep_alive(Duration::from_secs(self.mqtt.keep_alive))
        .set_last_will(LastWill::new(
            mqtt_config.last_will_topic(),
            mqtt_config.last_will_payload(),
            QoS::AtLeastOnce,
            true, // retain so that homeassistant knows this entity is offline even after restarting
        ))
        .to_owned();
        if let (Some(user), Some(password)) = (&self.mqtt.user, &self.mqtt.password) {
            mqttoptions.set_credentials(user, password);
        }

        AsyncClient::new(mqttoptions, 10)
    }

    pub fn build_switch(
        &self,
        command_topic: String,
        state_topic: String,
        availability: Availability,
        name: String,
        unique_id: String,
        value_template: String,
        json_attributes_template: String,
        payload_on: String,
        payload_off: String,
        json_attributes_topic: String,
    ) -> Switch {
        let common = ComponentCommon {
            name,
            unique_id,
            device: self.homeassistant.device.clone(),
            availability,
        };
        Switch {
            command_topic,
            state_topic: state_topic.clone(),
            common,
            value_template,
            json_attributes_topic,
            json_attributes_template,
            payload_on,
            payload_off,
            state_on: self.switch_on_value.clone(),
            state_off: self.switch_off_value.clone(),
            optimistic: false,
        }
    }
    pub fn build_select(
        &self,
        options: Vec<String>,
        command_topic: String,
        state_topic: String,
        availability: Availability,
        name: String,
        unique_id: String,
        value_template: String,
        json_attributes_template: String,
    ) -> Select {
        let common = ComponentCommon {
            name,
            unique_id,
            device: self.homeassistant.device.clone(),
            availability,
        };
        Select {
            command_topic,
            state_topic: state_topic.clone(),
            common,
            options,
            value_template,
            json_attributes_template,
            json_attributes_topic: state_topic,
        }
    }
    pub fn get_autodiscover_topic(&self, component: &impl Component) -> String {
        let component_str = component.component_str();
        let prefix = self.homeassistant.autodiscover_prefix.clone();
        let object_id = component.object_id();
        return format!("{prefix}/{component_str}/{object_id}/config");
    }
    pub async fn publish_autodiscover(&self, client: &AsyncClient, component: &impl Component) {
        let topic = self.get_autodiscover_topic(component);
        let payload = component.to_json();
        log::debug!(
            "Publishing autodiscover. topic: {}, payload: {}",
            &topic,
            &payload
        );
        client
            .publish(topic, QoS::AtLeastOnce, true, payload)
            .await
            .expect("publish autodiscover");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn config_can_construct() {
        let _ = super::Config::new();
    }
}
