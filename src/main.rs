#![windows_subsystem = "windows"]
#[macro_use]
extern crate enum_display_derive;
extern crate winrt_notification;

mod razer;
mod tpm2;
mod tray;
mod wled;

use palette::encoding::Srgb;
use palette::rgb::Rgb;
use palette::{FromColor, Hsluv, Hue};
use razer::Chroma;
use razer::RzType;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::slice::from_raw_parts;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use winrt_notification::{Duration as WinrtDuration, Sound, Toast};
use wled::{WLEDMessage, WLED};

use crate::tray::TrayMessage;

const DEFAULT_BAUD_RATE: u32 = 115_200;

static mut COLOR_PTR: Option<*const i32> = None;
static mut COLOR_STATE: ColorState = ColorState::Wave;

pub enum RenderMessage {
    Render(Vec<u8>),
    WLED(WLEDMessage),
    Quit,
}

#[derive(Clone, Copy)]
pub enum ColorState {
    Wave,
    Chroma,
}

unsafe extern "C" fn handle_events(kind: RzType, data: *const i32) {
    if let RzType::BroadcastStatus = kind {
        if data as i32 == 1 {
            COLOR_PTR = None;
            COLOR_STATE = ColorState::Chroma;
        } else if let None = COLOR_PTR {
            COLOR_STATE = ColorState::Wave;
        }

        return;
    } else {
        COLOR_PTR = Some(data);
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    razer_key: String,
    com_port: String,
    baud_rate: Option<u32>,
    dll: Option<String>,
}

impl ::std::default::Default for Config {
    fn default() -> Self {
        Self {
            razer_key: "".into(),
            com_port: "".into(),
            baud_rate: Some(DEFAULT_BAUD_RATE),
            dll: None,
        }
    }
}

fn main() {
    // Try to load config
    let cfg: Config = confy::load("rust-serial-controller", None).expect("Unable to load config.");

    // Try to connect to chroma
    let mut chroma = Chroma::init(
        guid::parse(cfg.razer_key.as_str()).expect("Invalid razer key GUID"),
        cfg.dll,
    );
    chroma.unregister_event_notification();
    chroma.register_event_notification(handle_events);

    let rate = if let Some(rate) = cfg.baud_rate {
        rate
    } else {
        DEFAULT_BAUD_RATE
    };

    // Determine the total number of LEDs
    let leds = wled::WLED::new(&cfg.com_port, rate).leds;
    println!("Got {} LEDS", leds);

    let toast = Toast::new(Toast::POWERSHELL_APP_ID).title("rust-serial-controller");

    if leds == 0 {
        toast
            .text1("No LEDs available.")
            .text2("Check your WLED configuration and reboot the microcontroller")
            .sound(Some(Sound::Default))
            .duration(WinrtDuration::Long)
            .show()
            .expect("Error notification failed");
        return;
    }

    toast
        .sound(Some(Sound::SMS))
        .text1(format!("Starting with {} LEDs", leds).as_str())
        .duration(WinrtDuration::Short)
        .show()
        .expect("Notification failed");

    // Set up a communication channel for rendering
    let (render_tx, render_rx): (Sender<RenderMessage>, Receiver<RenderMessage>) = mpsc::channel();

    // Spawn a thread to manage rendering
    let render_thread = thread::spawn(move || {
        render_loop(render_rx, wled::WLED::new(&cfg.com_port, rate)).unwrap();
    });

    // Configure the tray menu
    let (_tray, tray_rx) = tray::tray(render_tx.clone());

    // Start updating
    update_loop(leds, render_tx.clone(), tray_rx);

    // If we're here, update_loop has returned. So wait for the render_thread to finish.
    render_tx
        .send(RenderMessage::Quit)
        .expect("Failed to send Quit message to the render_thread.");
    render_thread.join().unwrap();
}

fn render_loop(rx: Receiver<RenderMessage>, mut wled: WLED) -> Result<(), wled::Error> {
    // 90 fps
    const RATE: Duration = Duration::from_millis(1000 / 90);

    let min_flush_rate = 100;
    let mut since = 0;

    // Start with a ping, that way if there's nothing specified to render we default to a ping loop at most once every min_flush_rate ms.
    let mut data: Vec<u8> = tpm2::ping();
    'main: loop {
        let mut new_data: Vec<u8> = data.clone();

        // Handle any render messages that might exist. Loop over any RenderMessages until we get to the end of the list so that we only handle the last.
        while let Ok(r) = rx.try_recv() {
            match r {
                RenderMessage::Render(data) => new_data = data,
                RenderMessage::WLED(message) => {
                    wled.send_message(message)?;
                }
                RenderMessage::Quit => break 'main,
            }
        }

        let mut needs_flush = since > min_flush_rate;

        // If we aren't already forced to flush, compare the last data sent to the new_data and only send if it isn't the same.
        if !needs_flush {
            for i in 0..data.len() {
                if new_data[i] != data[i] {
                    needs_flush = true;
                    break;
                }
            }
        }

        if needs_flush {
            data = new_data;
            wled.send_message(wled::WLEDMessage::Raw(data.clone()))?;
            since = 0;
        }

        thread::sleep(RATE);
        since += RATE.as_millis();
    }

    Ok(())
}

fn update_loop(leds: usize, tx: Sender<RenderMessage>, tray_rx: Receiver<TrayMessage>) {
    // Set the update rate to 240 fps
    const RATE: Duration = Duration::from_millis(1000 / 240);

    // Initialize color and colors deque
    let mut color = Hsluv::new(127.7, 100.0, 50.0);
    let mut colors: VecDeque<Rgb<Srgb, u8>> = VecDeque::from([Rgb::new(0_u8, 0_u8, 0_u8)]);

    // Fill colors deque with the initial color
    for _ in 1..leds {
        colors.push_back(colors[0]);
    }

    // Initialize the override_state variable
    let mut override_state: Option<ColorState> = None;

    // Start the update loop
    loop {
        // Process tray messages, if any
        match tray_rx.try_recv() {
            Ok(TrayMessage::Quit) => break,
            Ok(TrayMessage::OverrideState(state)) => override_state = Some(state),
            Err(_) => (),
        }

        // Determine the current state, considering the override_state if set
        let state = if let Some(state) = override_state {
            state
        } else {
            unsafe { COLOR_STATE }
        };

        // Update the colors based on the current state
        match state {
            ColorState::Wave => {
                // Shift the hue of the color
                color = color.shift_hue(0.25);
                let rgb_color: Rgb<Srgb, f32> = Rgb::from_color(color);

                // Update the colors deque
                colors.push_front(Rgb::new(
                    (rgb_color.red * 255.0) as u8,
                    (rgb_color.green * 255.0) as u8,
                    (rgb_color.blue * 255.0) as u8,
                ));

                // Make the colors deque contiguous and send the render message
                colors.make_contiguous();
                let (first, _) = colors.as_slices();
                println!("Got {}", first.len());
                tx.send(RenderMessage::Render(tpm2::pack(first))).unwrap();
            }
            ColorState::Chroma => {
                // If COLOR_PTR is set, process the color data
                if let Some(data) = unsafe { COLOR_PTR } {
                    let colors = unsafe { from_raw_parts(data as *const i32, 5) };

                    // Convert the color data into RGB colors
                    let color1 = color_from_data(colors[1]);
                    let color2 = color_from_data(colors[2]);
                    let color3 = color_from_data(colors[3]);
                    let color4 = color_from_data(colors[4]);

                    let min = leds / 4;
                    let max = leds - (min * 3) + 1;

                    // Create color segments and concatenate them
                    let mut c1 = vec![color1; min];
                    let mut c2 = vec![color2; min];
                    let mut c3 = vec![color3; min];
                    let mut c4 = vec![color4; max];

                    c1.append(&mut c2);
                    c1.append(&mut c3);
                    c1.append(&mut c4);

                    // Send the render message with the updated color data
                    tx.send(RenderMessage::Render(tpm2::pack(c1.as_slice())))
                        .expect("Failed to send tx");
                }
            }
        }

        // Sleep for the duration specified by the update rate
        thread::sleep(RATE);
    }
}

fn color_from_data(data: i32) -> Rgb<Srgb, u8> {
    Rgb::new(
        ((data >> 0) & 0xff) as u8,
        ((data >> 8) & 0xff) as u8,
        ((data >> 16) & 0xff) as u8,
    )
}
