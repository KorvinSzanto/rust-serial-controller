use std::{time::Duration, io::{Write, Read, self}};
use serialport::{COMPort, StopBits, SerialPort};

pub struct WLED {
    pub leds: usize,
    port: COMPort
}

impl WLED {

    pub fn new(com: &str) -> WLED {
        let mut serial = WLED {
            leds: 0,
            port: WLED::connect(com)
        };

        if let Ok(count) = serial.count_leds() {
            serial.leds = count;
        }

        serial
    }

    fn connect(com: &str) -> COMPort {
        serialport::new(com, 230_400)
            .timeout(Duration::from_millis(100))
            .stop_bits(StopBits::One)
            .open_native()
            .expect("Failed to open port")
    }

    fn count_leds(&mut self) -> Result<usize, Error> {
        let mut serial_buf: Vec<u8> = vec![];

        self.port.write_data_terminal_ready(true).expect("Unable to set terminal ready.");

        self.port.write(r#"l"#.as_bytes()).expect("Unable to write to port.");
        self.port.flush().expect("Failed to flush");

        if let Ok(_) = self.port.read_to_end(&mut serial_buf) {
            if serial_buf[0] == 91 {
                // let data = ((serial_buf[3] as u16) << 8) | (serial_buf[4] as u16);
                println!("GOT JSON DATA: {:?}", serial_buf);
            }
        };

        let mut count = 0;

        if serial_buf.len() > 2 {
            count = 1;
            for i in 0..serial_buf.len() {
                if serial_buf[i] == 44 {
                    count += 1;
                }
            }
        }

        self.port.write_data_terminal_ready(false)
            .expect("Unable to set terminal ready.");    

        Ok(count)
    }
    
    pub fn send_message(&mut self, msg: WLEDMessage) -> Result<usize, Error>  {
        let data = match msg {
            WLEDMessage::Raw(packet) => packet,
            WLEDMessage::On => r#"{"on":true}"#.as_bytes().to_vec(),
            WLEDMessage::Off => r#"{"on":false}"#.as_bytes().to_vec(),
            WLEDMessage::Toggle => r#"{"on":"t"}"#.as_bytes().to_vec(),
            WLEDMessage::AdjustBrightness(b) => format!(r#"{{"bri":{}}}"#, b).as_bytes().to_vec(),
        };

        let result = self.port.write(data.as_slice());
        if let Err(error) = result {
            return Err(Error::IO(error));
        }

        if let Err(error) = self.port.flush() {
            return Err(Error::IO(error));
        }

        Ok(result.unwrap())
    }
}

#[allow(dead_code)]
pub enum WLEDMessage {
    Raw(Vec<u8>),
    On,
    Off,
    Toggle,
    AdjustBrightness(u8),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    PortNotSet,
    IO(io::Error),
}