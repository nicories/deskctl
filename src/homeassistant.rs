// collection of fields common to all components
use serde::Deserialize;
use serde::Serialize;

pub trait HomeAssistantComponent {
    fn component_str(&self) -> &str;
    fn object_id(&self) -> &str;
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentCommon {
    pub name: String,
    pub object_id: String,
    pub unique_id: String,
    pub device: HomeAssistantDevice,
    pub availability: ComponentAvailability,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ComponentAvailability {
    pub payload_available: String,
    pub payload_not_available: String,
    pub topic: String,
}

#[derive(Serialize, Clone)]
pub struct ComponentSwitch {
    pub command_topic: String,
    pub state_topic: String,
    #[serde(flatten)]
    pub common: ComponentCommon,
    pub value_template: String,
    pub json_attributes_topic: String,
    pub json_attributes_template: String,
    pub payload_on: String,
    pub payload_off: String,
    pub state_on: String,
    pub state_off: String,
    pub optimistic: bool,
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

#[derive(Serialize, Clone)]
pub struct ComponentSelect {
    pub command_topic: String,
    pub state_topic: String,
    #[serde(flatten)]
    pub common: ComponentCommon,
    pub options: Vec<String>,
    pub value_template: String,
    pub json_attributes_topic: String,
    pub json_attributes_template: String,
}

impl HomeAssistantComponent for ComponentSelect {
    fn component_str(&self) -> &str {
        "select"
    }
    fn object_id(&self) -> &str {
        &self.common.object_id
    }
}
