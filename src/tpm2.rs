use palette::{rgb::Rgb, encoding::Srgb};

pub const TPM2_START: u8 = 0xC9;
const TPM2_TYPE_DATA: u8 = 0xDA;
const TPM2_TYPE_PING: u8 = 0xAA;
pub const TPM2_END: u8 = 0x36;

pub fn pack(colors: &[Rgb<Srgb, u8>]) -> Vec<u8> {
    pack_type(colors, TPM2_TYPE_DATA)
}

pub fn ping() -> Vec<u8> {
    pack_type(&[], TPM2_TYPE_PING)
}

pub fn pack_type(colors: &[Rgb<Srgb, u8>], kind: u8) -> Vec<u8> {
    let mut packet: Vec<u8> = vec![];
    
    // Add the header
    packet.push(TPM2_START);
    packet.push(kind);

    // Specify the length of the data
    let len: u16 = (colors.len() as u16) * 3;
    packet.push((len >> 8) as u8);
    packet.push((len & 0xFF) as u8);

    // Add colors
    for color in colors {
        packet.push(color.red);
        packet.push(color.green);
        packet.push(color.blue);
    }

    // Add the footer
    packet.push(TPM2_END);
    packet
}