# Rust serial controller
This package exists to connect a WLED enabled microcontroller connected via USB to Razer Chroma.

### Configuration
When you first run this project a configuration file should be created at the following location:

`%AppData%\rust-serial-controller\config\default-config.toml'`

- `razer_key` This is the razer chroma key. I used one of the keys listed in [ChromaBroadcastAPI](https://github.com/OpenChromaConnect/ChromaBroadcastAPI/blob/1800e6499dbaf557df397b98cfe38c15d05dbce2/inc/RzChromaBroadcastAPIDefines.h)
- `com_port` Set this to the COM_PORT your wled microcontroller is connected to. You can get a list of com ports in powershell using `Get-WMIObject Win32_SerialPort | Select-Object Name,DeviceID,Description`. Mine ended up being `COM6`
- **Optional**: `dll` Use this to specify the full path to a different razer chroma DLL. If not defined we'll default to `Razer/ChromaBroadcast/bin/RzChromaBroadcastAPI64.dll`

Any time you change these values you'll need to restart the program for changes to take effect.