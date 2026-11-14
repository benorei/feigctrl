#![warn(clippy::pedantic)]

use binrw::BinRead;
use std::io::{Cursor, Read};
use std::time::{Duration, SystemTime};
use crc::{Crc, CRC_8_MAXIM_DOW, CRC_32_JAMCRC};

use crate::ipp::*;

mod ipp;

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

    // Turn on light
    port.write_all(&build_message(0, 0x20, 4, vec![0x00, 0x00, 0x00, 0xC0])).unwrap();

    // Enable Type 11 cards
    // port.write_all(&build_message(0, 0xE4, 4, vec![0x00, 0x0B, 0x01, 0x01])).unwrap();

    // Enable DesFire cards
    port.write_all(&build_message(0, 0xE4, 4, vec![0x00, 0x07, 0x01, 0x01])).unwrap();

    // try beep
    // port.write_all(&buzzer_message(0x0016, 0x0001)).unwrap();


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
            Message::StatusReply(_) => {
                println!("{ipp_message:#x?}");

            }
            Message::Startup(_) => {
                println!("Startup");

            }
            Message::UnhandledCard(card_type, _data) => {
                println!("Unhandled Card: Type {card_type} (0x{card_type:x})"); 
            }
            Message::Heartbeat(_) => {
                // println!("<3 {}", ipp_message.sequence_number);
            }
            Message::Log(code, ascii) => {
                let text = String::from_utf8_lossy(&ascii);
                println!("Code {code:b}: {text}");
            }
            Message::EMVTransactionSuccessUnk(_) => {}
            Message::Unknown(data) => {
                println!("Unknown message type 0x{0:x}: {data:#x?}", ipp_message.message_type);
            }
            Message::ProxCardFunctionReply(fntype, _) => {println!("Enabled ProxCardFn {fntype}");}
            
            Message::DESFireRead(data) => {
                println!("DESFireRead: {data:#x?}");
            }
            Message::DESFireCardRemoved() => {println!("DESFireCardRemoved")}
            _ => {
                println!("{ipp_message:#x?}");
            }
        }

        // if clk.elapsed().unwrap().as_secs() >= 5 {
        //     let msg = build_message(
        //             ipp_message.sequence_number.wrapping_add(1),
        //             0x22,
        //             4,
        //             vec![0x06, 0x00, 0x01, 0x00]
        //     );
        //     port.write_all(&msg).unwrap();
        //     // println!("Sent: {:x?}", msg);

        //     // Turn on lights
        //     let mut msg2: Vec::<u8>;
        //     if toggle {
        //         msg2 = build_message(
        //             0, 0x20, 4, vec![0x00, 0x00, 0x00, 0x40]
        //         );
        //     } else {
        //         msg2 = build_message(
        //             0, 0x20, 4, vec![0x00, 0x00, 0x00, 0x80]
        //         );
        //     }
        //     port.write_all(&msg2).unwrap();
        //     toggle = !toggle;

        //     clk = SystemTime::now();
        // }
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
    return build_message(0, 0x20, 4, vec![frequency as u8, (frequency >> 8 as u8).try_into().unwrap(), duration as u8, (duration >> 8 as u8).try_into().unwrap()]);
}
