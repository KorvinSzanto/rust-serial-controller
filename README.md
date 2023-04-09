# Rust serial controller
This package exists to connect a WLED enabled microcontroller connected via USB to Razer Chroma.

### Configuration
When you first run this project a configuration file should be created in `%AppData%\rust-serial-controller\config\default-config.toml`

- `razer_key`: The Razer Chroma key. Use one of the keys found in [ChromaBroadcastAPI](https://github.com/OpenChromaConnect/ChromaBroadcastAPI/blob/1800e6499dbaf557df397b98cfe38c15d05dbce2/inc/RzChromaBroadcastAPIDefines.h)
- `com_port`: The COM port to which your WLED microcontroller is connected. You can obtain a list of available COM ports in PowerShell using `Get-WMIObject Win32_SerialPort | Select-Object Name,DeviceID,Description`.
- `baud_rate` (optional): Use this to set a custom baud_rate. The default is `115_200`.
- `dll` (optional): Use this to specify the full path to an alternative Razer Chroma DLL. The default path is `Razer/ChromaBroadcast/bin/RzChromaBroadcastAPI64.dll`.

Any time you change these values you'll need to restart the program for changes to take effect.