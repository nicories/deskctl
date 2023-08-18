# Desktop Utility

Control your PC from Home-Assistant (or any other MQTT hubs)

# Features

- PulseAudio
  - select default output device
  - increase/decrease volume
  - mute
- Sway
  - enable/disable displays
- Custom Scripts

# Issues

## Display Commands don't work

modprobe i2c-dev

DDC/CI is just awful and the support is very hit or miss depending on the manufacturer and model of the display and the OS/drivers. 