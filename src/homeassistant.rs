// collection of fields common to all components
use serde::Deserialize;
use serde::Serialize;

pub trait Component {
    fn component_str(&self) -> &str;
    fn object_id(&self) -> &str;
    fn to_json(&self) -> String;
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ComponentCommon {
    pub name: String,
    pub unique_id: String,
    pub device: Device,
    pub availability: Availability,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Availability {
    pub payload_available: String,
    pub payload_not_available: String,
    pub topic: String,
}

#[derive(Serialize, Clone)]
pub struct Switch {
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
impl Component for Switch {
    fn component_str(&self) -> &str {
        "switch"
    }

    fn object_id(&self) -> &str {
        &self.common.unique_id
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Device {
    name: String,
    identifiers: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct Select {
    pub command_topic: String,
    pub state_topic: String,
    #[serde(flatten)]
    pub common: ComponentCommon,
    pub options: Vec<String>,
    pub value_template: String,
    pub json_attributes_topic: String,
    pub json_attributes_template: String,
}

impl Component for Select {
    fn component_str(&self) -> &str {
        "select"
    }
    fn object_id(&self) -> &str {
        &self.common.unique_id
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
