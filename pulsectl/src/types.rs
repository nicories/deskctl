use serde::{Deserialize, Serialize};
use std::{collections::HashMap, string::FromUtf8Error};

#[derive(Deserialize, Debug)]
pub enum EventType {
    Change,
    Remove,
    Unknown,
}
#[derive(Deserialize, Debug)]
pub enum EventTarget {
    Client,
    Sink,
    Unknown,
}
#[derive(Deserialize, Debug)]
pub struct PulseEvent {
    pub event_type: EventType,
    pub target: EventTarget,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Volume {
    pub value: u32,
    pub value_percent: String,
    pub db: String,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ServerInfo {
    pub server_string: String,
    pub default_sink_name: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SinkInfo {
    pub index: u32,
    pub state: String,
    pub name: String,
    pub mute: bool,
    // channels, comma-seperated
    pub channel_map: String,
    // channel -> Volume
    pub volume: HashMap<String, Volume>,
}

#[derive(Debug)]
pub enum Error {
    JsonError(serde_json::Error),
    PulseError(String),
    Utf8Error(FromUtf8Error),
}
impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Self::Utf8Error(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonError(e)
    }
}
