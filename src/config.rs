use std::time::Duration;

use rumqttc::AsyncClient;
use rumqttc::EventLoop;
use rumqttc::LastWill;
use rumqttc::MqttOptions;
use rumqttc::QoS;
use serde::Deserialize;
use serde::Serialize;

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
pub struct ComponentAvailability {
    pub payload_available: String,
    pub payload_not_available: String,
    pub topic: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentSwitch {
    pub command_topic: String,
    pub availability: ComponentAvailability,
    pub state_topic: String,
    #[serde(flatten)]
    pub common: ComponentCommon,
    value_template: String,
    json_attributes_topic: String,
    json_attributes_template: String,
}
impl HomeAssistantComponent for ComponentSwitch {
    fn component_str(&self) -> &str {
        "switch"
    }

    fn object_id(&self) -> &str {
        &self.common.object_id
    }
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct HomeAssistantDevice {
    name: String,
    identifiers: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentSelect {
    pub command_topic: String,
    pub availability: ComponentAvailability,
    pub state_topic: String,
    #[serde(flatten)]
    pub common: ComponentCommon,
    #[serde(default = "Vec::new")]
    pub options: Vec<String>,
    value_template: String,
    json_attributes_topic: String,
    json_attributes_template: String,
}

impl HomeAssistantComponent for ComponentSelect {
    fn component_str(&self) -> &str {
        "select"
    }
    fn object_id(&self) -> &str {
        &self.common.object_id
    }
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
// collection of fields common to all components
#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentCommon {
    pub name: String,
    pub object_id: String,
    pub unique_id: String,
    pub device: HomeAssistantDevice,
}

impl ComponentCommon {
    pub fn new(
        name: String,
        object_id: String,
        unique_id: String,
        device: HomeAssistantDevice,
    ) -> Self {
        Self {
            name,
            object_id,
            unique_id,
            device,
        }
    }
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct HomeAssistantConfig {
    pub autodiscover: bool,
    /// prefix used in homeassistant discovery, see https://www.home-assistant.io/integrations/mqtt/#discovery-options
    pub autodiscover_prefix: String,
    // pub autodiscover_topic: String,
    pub device: HomeAssistantDevice,
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
    pub availability: ComponentAvailability,
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
    pub mqtt_name: String,
    pub outputs_state_topic: String,
    pub outputs_command_topic: String,
    pub outputs_attributes_topic: String,
    pub outputs_attributes_template: String,
    pub outputs_value_template: String,
    pub state_topic: String,
    pub command_topic: String,
    pub availability: ComponentAvailability,
    pub workspaces_select: ComponentSelect,
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
    pub queue_size: usize,
    pub mqtt: MqttConfig,
    pub homeassistant: HomeAssistantConfig,
    pub pulseaudio: PulseAudioConfig,
    pub sway: SwayConfig,
    // scripts: Vec<ScriptConfig>,
}

pub trait HomeAssistantComponent {
    fn component_str(&self) -> &str;
    fn object_id(&self) -> &str;
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
        let mqttoptions = MqttOptions::new(
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

        AsyncClient::new(mqttoptions, self.queue_size)
    }

    pub fn build_switch(
        &self,
        command_topic: String,
        state_topic: String,
        availability: ComponentAvailability,
        name: String,
        object_id: String,
        unique_id: String,
        value_template: String,
        json_attributes_template: String,
    ) -> ComponentSwitch {
        let common = ComponentCommon {
            name,
            object_id,
            unique_id,
            device: self.homeassistant.device.clone(),
        };
        ComponentSwitch {
            command_topic,
            availability,
            state_topic: state_topic.clone(),
            common,
            value_template,
            json_attributes_topic: state_topic,
            json_attributes_template,
        }
    }
    pub fn build_select(
        &self,
        options: Vec<String>,
        command_topic: String,
        state_topic: String,
        availability: ComponentAvailability,
        name: String,
        object_id: String,
        unique_id: String,
        value_template: String,
        json_attributes_template: String,
    ) -> ComponentSelect {
        let common = ComponentCommon {
            name,
            object_id,
            unique_id,
            device: self.homeassistant.device.clone(),
        };
        ComponentSelect {
            command_topic,
            state_topic: state_topic.clone(),
            availability,
            common,
            options,
            value_template,
            json_attributes_template,
            json_attributes_topic: state_topic,
        }
    }
    pub fn get_autodiscover_topic(&self, component: &dyn HomeAssistantComponent) -> String {
        let component_str = component.component_str();
        let prefix = self.homeassistant.autodiscover_prefix.clone();
        let object_id = component.object_id();
        format!("{prefix}/{component_str}/{object_id}/config")
    }
}

mod test {
    use super::*;

    #[test]
    fn config_can_construct() {
        let _ = Config::new();
    }
}
