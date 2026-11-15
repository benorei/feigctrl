#![warn(clippy::pedantic)]

use binrw::{BinRead};
use std::io::{Cursor, Read};
use std::time::{SystemTime};
use crc::{Crc, CRC_8_MAXIM_DOW, CRC_32_JAMCRC};

use crate::ipp::*;
use crate::ntag::*;

mod ipp;
mod ntag;

pub const HDR_CRC: Crc::<u8> = Crc::<u8>::new(&CRC_8_MAXIM_DOW);
pub const DATA_CRC: Crc::<u32> = Crc::<u32>::new(&CRC_32_JAMCRC);

fn main() {
    let mut port = serialport::new("/dev/ttyS3", 115_200)
        .timeout(std::time::Duration::MAX)
        .open()
        .expect("Failed to open serial port");
    
    println!("Port opened");

    // Max payload size + data CRC size + header size w/ one byte of extra padding
    let mut current_message = vec![0u8; 65536 + 4 + 8].into_boxed_slice();

    let mut clk = SystemTime::now();
    let mut lights_on = true;

    // Send reboot
    // port.write_all(&build_message(0, 0x10, 1, vec![0x03])).unwrap();

    // Turn on light
    port.write_all(&lights_message(true, true)).unwrap();    

    // Enable Type 10 cards
    port.write_all(&build_message(0, 0xE4, 4, vec![0x00, 0x0A, 0x01, 0x01])).unwrap();

    // Enable DesFire cards
    // port.write_all(&build_message(0, 0xE4, 4, vec![0x00, 0x07, 0x01, 0x01])).unwrap();

    // Enable ISO14443A cards
    // port.write_all(&build_message(0, 0xE4, 4, vec![0x00, 0x06, 0x01, 0x01])).unwrap();

    loop {
        if port.read(&mut current_message).is_err() {
            println!("Read call failed!");
            continue;
        }

        let mut out_cursor = Cursor::new(&current_message);
        let Ok(ipp_message) = IppMessage::read(&mut out_cursor) else {
            println!("Received non-IPP message over serial!");
            continue;
        };

        match ipp_message.data {
            Message::VersionReply(_) => {
                println!("{ipp_message:#x?}");
            }
            Message::StatusReply(sr) => {
                if sr[0] != 0x0 || sr[1] != 0x0 {println!("StatusReply {:#x}, {:#x}", sr[0], sr[1])};
            }
            Message::ResetReply(rr) => {
                println!("ResetReply {:#x}, {:#x}", rr[0], rr[1]);
            }
            Message::Startup(_) => {
                println!("Startup");
            }
            Message::UnhandledCard(card_type, _data) => {
                println!("Unhandled Card: Type {card_type} (0x{card_type:x})"); 
                port.write_all(&Message::Buzzer(6, 1).to_ipp()).unwrap();
            }
            Message::Heartbeat(_) => {
                port.write_all(&lights_message(lights_on, !lights_on)).unwrap();
                lights_on = !lights_on;
            }
            Message::Log(code, ascii) => {
                let text = String::from_utf8_lossy(&ascii);
                println!("Code {code:b}: {text}");
            }
            Message::Unknown(data) => {
                println!("Unknown message type 0x{0:x}: {data:#x?}", ipp_message.message_type);
            }
            Message::DESFireRead(data) => {
                println!("DESFireRead: {data:#x?}");
            }
            Message::DESFireCardRemoved() => {println!("DESFireCardRemoved")}
            Message::EMVStatus(_) => {}
            Message::ProxCardFunctionReply(card_type, data) => {println!("PCFR Type: 0x{card_type:x} Data: {data:#x?}");}

            Message::UltralightReply(cmd, status, payload) => {
                println!("UltralightReply CMD: {cmd:x}, Status: {status:x}, Payload: {payload:#x?}");

                // Send UltralightCommand
                if cmd == 0 {
                    let uc = build_message(0x0, 0xd4, 3, vec![0x02, 0, 135]);
                    // let uc = build_message(0x0, 0xd4, 3, vec![0x02, 4, 126]); // Read only user memory portion
                    port.write_all(&uc).unwrap();
                }

                if cmd == 2 { // Read response
                    let mut cursor = Cursor::new(payload);
                    let _header = NTAGHeader::read_le(&mut cursor).unwrap();
                    let record: NTAGRecord = NTAGRecord::read_le(&mut cursor).unwrap();
                    // println!("Contents: {:#x?}", String::from_utf8_lossy(&record.payload.));
                    match record.payload {
                        NTAGPayload::Text { lang, txt } => {
                            println!("lang: {}, txt: {}", String::from_utf8_lossy(&lang), String::from_utf8_lossy(&txt));
                        }
                    }
                }
            }
            _ => {
                println!("{ipp_message:#x?}");
            }
        }

        if clk.elapsed().unwrap().as_secs() >= 5 {
            port.write_all(&status_message()).unwrap();
            clk = SystemTime::now();
        }
    }
}

// Contents should include message crc if necessary
pub fn build_message(seq:u8, msgtype:u8, len:u16, contents:Vec<u8>) -> Vec<u8> {
    let mut msg = vec![0xBC, seq, 0x0, msgtype, (len >> 8 as u8).try_into().unwrap(), len as u8, ];
    msg.push(HDR_CRC.checksum(&msg));
    msg.extend_from_slice(&contents);
    if msgtype >= 0x80 {msg.extend_from_slice(&DATA_CRC.checksum(&contents).to_le_bytes())};
    return msg;
}

pub fn buzzer_message(frequency: u16, duration:u16) -> Vec<u8> {
    return build_message(0, 0x22, 4, 
        vec![frequency as u8, (frequency >> 8 as u8).try_into().unwrap(), duration as u8, (duration >> 8 as u8).try_into().unwrap()]
    );
    
}

pub fn lights_message(left: bool, right: bool) -> Vec<u8> {
    let mut bitmap: u32 = 0x0;
    if left {bitmap += 0x40;}
    if right {bitmap += 0x80;}
    return build_message(0, 0x20, 4, bitmap.to_be_bytes().to_vec());
}

pub fn status_message() -> Vec<u8> {
    return build_message(0, 0x04, 0, vec![]);
}

// Convert Message enum into vec for writing
// pub fn build_message2(contents: Message) -> Vec<u8> {
//     let mut writer = Cursor::new(Vec::new());
//     contents.write_le(&mut writer).unwrap();
//     let bin = writer.into_inner();

//     // Build ippmessage. use sequence num 0
//     let mut msg = vec![0xBC, 0x0, 0x0, msgtype, (bin.len() >> 8 as u8).try_into().unwrap(), bin.len() as u8, ];
//     msg.push(HDR_CRC.checksum(&msg));
//     msg.extend_from_slice(&bin);
//     if msgtype >= 0x80 {msg.extend_from_slice(&DATA_CRC.checksum(&bin).to_le_bytes())};
//     return msg;
// }