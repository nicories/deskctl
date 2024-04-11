use ddc_hi::{Ddc, Display};

fn input_value_to_string(val: u8) -> &'static str {
    match val {
        0x0f => "DisplayPort 1",
        0x10 => "DisplayPort 2",
        0x11 => "HDMI 1",
        0x12 => "HDMI 2",
        _ => "Unknown",
    }
}


fn main() {
    for mut display in Display::enumerate() {
        display.update_capabilities().unwrap();
        println!(
            "{:?} {}: {:?} {:?}",
            display.info.backend,
            display.info.id,
            display.info.manufacturer_id,
            display.info.model_name
        );
        let cap = display.handle.capabilities().unwrap();
        let input_values = cap.vcp_features.get(&0x60).unwrap();
        for i in input_values.values() {
            dbg!(i);
        }
    }
}
