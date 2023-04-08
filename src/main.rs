#![windows_subsystem = "windows"]
#[macro_use]
extern crate enum_display_derive;

mod razer;
mod tpm2;
mod wled;

use razer::Chroma;
use palette::encoding::Srgb;
use palette::rgb::Rgb;
use palette::{FromColor, Hsluv, Hue};
use razer::RzType;
use tray_item::TrayItem;
use wled::{WLEDMessage, WLED};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use serde::{Serialize, Deserialize};

use std::slice::from_raw_parts;

static mut COLOR_PTR: Option<*const i32> = None;
static mut COLOR_STATE: ColorState = ColorState::Wave;

enum TrayMessage {
    Quit,
    OverrideState(ColorState),
}

enum RenderMessage {
    Render(Vec<u8>),
    Quit,
    WLED(WLEDMessage),
}

#[derive(Clone, Copy)]
enum ColorState {
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
    dll: Option<String>,
}

impl ::std::default::Default for Config {
    fn default() -> Self { Self { razer_key: "".into(), com_port: "".into(), dll: None } }
}

fn main() {
    let cfg: Config = confy::load("rust-serial-controller", None).expect("Unable to load config.");
    println!("{:?}", cfg);

    let (render_tx, render_rx): (Sender<RenderMessage>, Receiver<RenderMessage>) = mpsc::channel();

    let mut chroma = Chroma::init(guid::parse(cfg.razer_key.as_str()).expect("Invalid razer key GUID"), cfg.dll);
    chroma.unregister_event_notification();
    chroma.register_event_notification(handle_events);

    let leds = { wled::WLED::new(&cfg.com_port).leds };
    println!("Got {} LEDS", leds);

    let render_thread = thread::spawn(move || {
        render_loop(render_rx, wled::WLED::new(&cfg.com_port)).unwrap();
    });

    let (_tray, tray_rx) = tray(render_tx.clone());

    update_loop(leds, render_tx.clone(), tray_rx);
    render_thread.join().unwrap();
}

fn tray(render_tx: Sender<RenderMessage>) -> (TrayItem, Receiver<TrayMessage>) {
    let (tray_tx, tray_rx) = mpsc::channel();

    let mut tray = TrayItem::new("Rust WLED/Chroma Controller", "rust-serial-controller").unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Toggle", move || {
        send.send(RenderMessage::WLED(WLEDMessage::Toggle)).unwrap();
    }).unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 25%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(64))).unwrap();
    }).unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 50%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(128))).unwrap();
    }).unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 75%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(192))).unwrap();
    }).unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 100%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(255))).unwrap();
    }).unwrap();

    let send = tray_tx.clone();
    tray.add_menu_item("Effect: Wave", move || {
        send.send(TrayMessage::OverrideState(ColorState::Wave)).unwrap();
    }).unwrap();

    let send = tray_tx.clone();
    tray.add_menu_item("Effect: Chroma", move || {
        send.send(TrayMessage::OverrideState(ColorState::Wave)).unwrap();
    }).unwrap();

    let send = tray_tx.clone();
    let render_send = render_tx.clone();
    tray.add_menu_item("Quit", move || {
        send.send(TrayMessage::Quit).unwrap();
        render_send.send(RenderMessage::Quit).unwrap();
    }).unwrap();
    (tray, tray_rx)
}

fn render_loop(rx: Receiver<RenderMessage>, mut wled: WLED) -> Result<(), wled::Error> {
    const RATE: Duration = Duration::from_millis(1000 / 60);
    
    let mut data: Vec<u8> = tpm2::ping();
    let mut since = 0;
    'main: loop {
        let mut needs_flush = since > 100;
        let mut new_data: Vec<u8> = data.clone();
        while let Ok(r) = rx.try_recv() {
            match r {
                RenderMessage::Render(data) => new_data = data,
                RenderMessage::Quit => break 'main,
                RenderMessage::WLED(message) => {
                    wled.send_message(message)?;
                },
            }
        }

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
            wled.send_message(wled::WLEDMessage::Raw(data.clone()))? ;
            since = 0;
        }
  
        thread::sleep(RATE);
        since += RATE.as_millis();
    }

    Ok(())
}

fn update_loop(leds: usize, tx: Sender<RenderMessage>, tray_rx: Receiver<TrayMessage>) {
    const RATE: Duration = Duration::from_millis(1000 / 60);
    
    let mut color = Hsluv::new(127.7, 100.0, 50.0);
    let mut colors: VecDeque<Rgb<Srgb, u8>> = VecDeque::from([Rgb::new(0_u8, 0_u8, 0_u8)]);

    for _ in 1..leds {
        colors.push_back(colors[0]);
    }

    let mut override_state: Option<ColorState> = None;
    loop {
        match tray_rx.try_recv() {
            Ok(TrayMessage::Quit) => break,
            Ok(TrayMessage::OverrideState(state)) => override_state = Some(state),
            Err(_) => ()
        }

        let state = if let Some(state) = override_state { state } else { unsafe { COLOR_STATE } };
        match state {
            ColorState::Wave => {
                color = color.shift_hue(1.0);
                let rgb_color: Rgb<Srgb, f32> = Rgb::from_color(color);

                colors.pop_back();
                colors.push_front(Rgb::new(
                    (rgb_color.red * 255.0) as u8,
                    (rgb_color.green * 255.0) as u8,
                    (rgb_color.blue * 255.0) as u8,
                ));

                colors.make_contiguous();
                let (first, _) = colors.as_slices();
                tx.send(RenderMessage::Render(tpm2::pack(first))).unwrap();
            }
            ColorState::Chroma => {
                if let Some(data) = unsafe { COLOR_PTR } {
                    let colors = unsafe { from_raw_parts(data as *const i32, 5) };

                    let color1 = color_from_data(colors[1]);
                    let color2 = color_from_data(colors[2]);
                    let color3 = color_from_data(colors[3]);
                    let color4 = color_from_data(colors[4]);

                    let low = leds / 4;

                    let mut c1 = vec![color1; low];
                    let mut c2 = vec![color2; low];
                    let mut c3 = vec![color3; low];
                    let mut c4 = vec![color4; leds - (low * 3) + 1];

                    c1.append(&mut c2);
                    c1.append(&mut c3);
                    c1.append(&mut c4);

                    tx.send(RenderMessage::Render(tpm2::pack(c1.as_slice()))).expect("Failed to send tx");
                }
            }
        }
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
