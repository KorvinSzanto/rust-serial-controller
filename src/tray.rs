use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use tray_item::TrayItem;

use crate::wled::WLEDMessage;
use crate::{ColorState, RenderMessage};

pub enum TrayMessage {
    Quit,
    OverrideState(ColorState),
}

pub fn tray(render_tx: Sender<RenderMessage>) -> (TrayItem, Receiver<TrayMessage>) {
    let (tray_tx, tray_rx) = mpsc::channel();

    let mut tray = TrayItem::new("Rust WLED/Chroma Controller", "rust-serial-controller").unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Toggle", move || {
        send.send(RenderMessage::WLED(WLEDMessage::Toggle)).unwrap();
    })
    .unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 25%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(64)))
            .unwrap();
    })
    .unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 50%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(128)))
            .unwrap();
    })
    .unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 75%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(192)))
            .unwrap();
    })
    .unwrap();

    let send = render_tx.clone();
    tray.add_menu_item("Brightness 100%", move || {
        send.send(RenderMessage::WLED(WLEDMessage::AdjustBrightness(255)))
            .unwrap();
    })
    .unwrap();

    let send = tray_tx.clone();
    tray.add_menu_item("Effect: Wave", move || {
        send.send(TrayMessage::OverrideState(ColorState::Wave))
            .unwrap();
    })
    .unwrap();

    let send = tray_tx.clone();
    tray.add_menu_item("Effect: Chroma", move || {
        send.send(TrayMessage::OverrideState(ColorState::Chroma))
            .unwrap();
    })
    .unwrap();

    let send = tray_tx.clone();
    let render_send = render_tx.clone();
    tray.add_menu_item("Quit", move || {
        send.send(TrayMessage::Quit).unwrap();
        render_send.send(RenderMessage::Quit).unwrap();
    })
    .unwrap();

    (tray, tray_rx)
}
