use async_process::{Command, Stdio};
use async_stream::stream;
use futures_lite::{io::BufReader, prelude::*};
use tokio_stream::Stream;

mod types;
pub use types::*;

pub struct Pulseaudio {
    client_name: String,
}
impl Pulseaudio {
    pub fn new(client_name: &str) -> Self {
        Self {
            client_name: client_name.to_owned(),
        }
    }
    pub async fn subscribe(&self) -> impl Stream<Item = PulseEvent> {
        let mut child = Command::new("pactl")
            .arg("--format")
            .arg("json")
            .arg("subscribe")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        let s = stream! {
            loop {
                let mut buf = Vec::new();
                let _ = reader.read_until("}".as_bytes()[0], &mut buf).await;
                let s = String::from_utf8(buf).unwrap();
                let data: serde_json::Value = serde_json::from_str(&s).unwrap();
                let event_type = if data["event"] == "change" { EventType::Change }
                    else if data["event"] == "remove" { EventType::Remove }
                    else {EventType::Unknown};

                let target = if data["on"] == "sink" { EventTarget::Sink }
                    else if data["on"] == "client" { EventTarget::Client }
                    else { EventTarget::Unknown };
                let event = PulseEvent {
                    event_type,
                    target,
                };


            yield event;
            }
        };
        s
    }
    async fn run_command_with_output<T: serde::de::DeserializeOwned>(
        &self,
        command: &str,
    ) -> Result<T, Error> {
        let cmd = "--format json ".to_owned() + command;
        let args = cmd.split_whitespace();
        let output = async_process::Command::new("pactl")
            .args(["--client-name", &self.client_name])
            .args(args)
            .output()
            .await
            .expect("error running pactl");
        let json_string = String::from_utf8(output.stdout)?;
        let result = serde_json::from_str(&json_string)?;
        Ok(result)
    }
    async fn run_command(&self, command: &str) -> Result<(), Error> {
        assert!(!command.is_empty());
        let args = command.split_whitespace();
        let output = async_process::Command::new("pactl")
            .args(["--client-name", &self.client_name])
            .args(args)
            .output()
            .await
            .expect("error running pactl");
        if output.status.success() {
            Ok(())
        } else {
            // at the time of writing, the error message is a simple string
            Err(Error::PulseError(String::from_utf8(output.stdout)?))
        }
    }
    pub async fn set_default_sink(&self, sink: &SinkInfo) -> Result<(), Error> {
        let cmd = String::from("set-default-sink ") + &sink.name;
        self.run_command(&cmd).await
    }
    pub async fn server_info(&self) -> Result<ServerInfo, Error> {
        self.run_command_with_output("info").await
    }
    pub async fn list_sinks(&self) -> Result<Vec<SinkInfo>, Error> {
        let sinks: Vec<SinkInfo> = self.run_command_with_output("list sinks").await?;
        Ok(sinks)
    }
    pub async fn find_sink_by_name(&self, name: &str) -> Option<SinkInfo> {
        let sinks = self.list_sinks().await.ok()?;
        for s in sinks {
            if s.name == name {
                return Some(s);
            }
        }
        None
    }
    pub async fn get_default_sink(&self) -> Result<SinkInfo, Error> {
        let default_sink_name = self.server_info().await?.default_sink_name;
        let sinks = self.list_sinks().await?;
        let default_sink = sinks
            .into_iter()
            .filter(|s| s.name == default_sink_name)
            .last()
            .ok_or(Error::PulseError("could not get default sink".to_owned()));
        default_sink
    }
    // pactl actually has an easier command for this:
    // `pactl --format json get-sink-volume @DEFAULT_SINK@`
    // but that doesn't output json as of pactl v16.1 so it's annoying to parse
    // TODO: check back for future versions
    pub async fn get_default_volume(&self) -> Result<Volume, Error> {
        let default_sink = self.get_default_sink().await?;
        let volume = default_sink
            .volume
            .get("front-left")
            .ok_or(Error::PulseError("channel does not exist".to_owned()))?;
        Ok(volume.clone())
    }
    pub async fn volume_up(&self, step: u8) -> Result<(), Error> {
        self.run_command(&("set-sink-volume @DEFAULT_SINK@ +".to_owned() + &step.to_string() + "%"))
            .await
    }
    pub async fn volume_down(&self, step: u8) -> Result<(), Error> {
        self.run_command(&("set-sink-volume @DEFAULT_SINK@ -".to_owned() + &step.to_string() + "%"))
            .await
    }
    pub async fn toggle_mute(&self) -> Result<(), Error> {
        self.run_command("set-sink-mute @DEFAULT_SINK@ toggle")
            .await
    }
    pub async fn cycle_sinks(&self) -> Result<(), Error> {
        let sinks = self.list_sinks().await?;
        let default_sink_name = self.server_info().await?.default_sink_name;
        let sinks_count = sinks.len();
        for (i, sink) in sinks.iter().enumerate() {
            if sink.name == default_sink_name {
                let new_sink = sinks.get((i + 1) % sinks_count).expect("error lmao");
                self.set_default_sink(&new_sink).await?;
                return Ok(());
            }
        }
        Err(Error::PulseError("Couldn't cycle sinks".to_owned()))
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use tokio_test::block_on;

    use super::*;

    const TEST_CLIENT_NAME: &str = "test-client";

    #[test]
    fn run_error_cmd() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        {
            let result = block_on(pulse.run_command("blub"));
            let error = result.err().unwrap();
            assert!(matches!(error, Error::PulseError(_)));
        }
        {
            let result = block_on(pulse.run_command("list abcde"));
            let error = result.err().unwrap();
            assert!(matches!(error, Error::PulseError(_)));
        }
    }
    #[test]
    fn list_sinks() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        let sinks = block_on(pulse.list_sinks()).unwrap();
        assert!(sinks.len() > 0);
    }
    #[test]
    fn get_default_volume_string() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        let volume = block_on(pulse.get_default_volume()).unwrap().value_percent;
        assert!(!volume.is_empty());
        assert!(volume.contains("%"));
    }
    #[test]
    #[serial_test::serial]
    fn volume_up_down() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        let initial_volume = block_on(pulse.get_default_volume()).unwrap().value;
        block_on(pulse.volume_up(5)).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        let higher_volume = block_on(pulse.get_default_volume()).unwrap().value;
        assert!(higher_volume > initial_volume);
        block_on(pulse.volume_down(5)).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        assert!(block_on(pulse.get_default_volume()).unwrap().value < higher_volume);
    }
    #[test]
    #[serial_test::serial]
    fn toggle_mute() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        let muted_initial = block_on(pulse.get_default_sink()).unwrap().mute;
        block_on(pulse.toggle_mute()).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        assert!(muted_initial != block_on(pulse.get_default_sink()).unwrap().mute);

        block_on(pulse.toggle_mute()).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        assert!(muted_initial == block_on(pulse.get_default_sink()).unwrap().mute);
    }

    // #[ignore = "endless"]
    #[test]
    #[serial_test::serial]
    fn subscribe() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        let stream = block_on(pulse.subscribe());
        pin_mut!(stream);
        while let Some(s) = block_on(stream.next()) {
            dbg!(s);
        }
    }
    #[ignore = "annoying"]
    #[serial_test::serial]
    #[test]
    fn set_default_sink() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        for sink in block_on(pulse.list_sinks()).unwrap() {
            block_on(pulse.set_default_sink(&sink)).unwrap();
            std::thread::sleep(Duration::from_secs(1));
            assert!(block_on(pulse.server_info()).unwrap().default_sink_name == sink.name);
        }
    }
    #[ignore = "annoying"]
    #[test]
    #[serial_test::serial]
    fn cycle_sinks() {
        let pulse = Pulseaudio::new(TEST_CLIENT_NAME);
        let sinks = block_on(pulse.list_sinks()).unwrap();
        let count = sinks.len();
        let first_sink = block_on(pulse.get_default_sink()).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        for _ in 0..count - 1 {
            block_on(pulse.cycle_sinks()).unwrap();
            std::thread::sleep(Duration::from_secs(1));
            assert_ne!(
                first_sink.name,
                block_on(pulse.get_default_sink()).unwrap().name
            );
        }
        // should be at the first one now
        block_on(pulse.cycle_sinks()).unwrap();
        assert_eq!(
            first_sink.name,
            block_on(pulse.get_default_sink()).unwrap().name
        );
    }
}
