use env_logger::Env;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ComponentAvailability {
    pub payload_available: String,
    pub payload_not_available: String,
    pub topic: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentSwitch {
    #[serde(flatten)]
    pub common: ComponentCommon,
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
    // pub sink_select: ComponentSelect,
    pub availability: ComponentAvailability,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct SwayConfig {
    pub state_topic: String,
    pub command_topic: String,
    pub availability: ComponentAvailability,
    pub workspaces_select: ComponentSelect,
}
#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub app_name: String,
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

static CONFIG_STR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/default_config.yaml"
));

impl Config {
    pub fn new() -> Self {
        // construct config and add it to the template environment
        let config: Config = serde_yaml::from_str(CONFIG_STR).expect("toml default config");

        config
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
