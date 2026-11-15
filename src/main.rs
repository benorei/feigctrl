#![warn(clippy::pedantic)]

use binrw::{BinRead};
use std::io::{Cursor, Read};
use std::time::{SystemTime};

use crate::ipp::*;
use crate::ntag::*;

mod ipp;
mod ntag;

const IGNORE_ERRS: [u8; 2] = [2, 3];

// TODO
// - Enum for UltralightCommand/UltralightReply (incl enter & leave field) for pattern matching
// - Decide if I want to support writes
// - Expose Message-based reader/writer

fn main() {

    let mut port = serialport::new("/dev/ttyS3", 115_200)
        .timeout(std::time::Duration::MAX)
        .open()
        .expect("Failed to open serial port");
    
    println!("Port opened");

    // Max payload size + data CRC size + header size w/ one byte of extra padding
    let mut current_message = vec![0u8; 65536 + 4 + 8].into_boxed_slice();

    let mut clk = SystemTime::now();
    let mut lights_on = false;

    // Send reboot
    // port.write_all(&build_message(0, 0x10, 1, vec![0x03])).unwrap();

    // Enable Type 10 cards
    port.write_all(&Message::ProxCardFunction(10, 1).to_ipp()).unwrap();

    // Enable DesFire cards
    // port.write_all(&build_message(0, 0xE4, 4, vec![0x00, 0x07, 0x01, 0x01])).unwrap();

    loop {
        if port.read(&mut current_message).is_err() {
            println!("Read call failed!");
            continue;
        }

        let mut out_cursor = Cursor::new(&current_message);
        let Ok(ipp_message) = IppMessage::read(&mut out_cursor) else {
            // println!("Received non-IPP message over serial!");
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
                // let val: u32;
                // if lights_on {val = LIGHTS_ON} else {val = LIGHTS_OFF};
                // let msg = Message::Lights(val).to_ipp();
                let msg: Message;
                if lights_on {msg = Message::LightsOff()} else {msg = Message::LightsOn()}
                port.write_all(&msg.to_ipp()).unwrap();
                lights_on = !lights_on;
            }
            Message::Log(code, ascii) => {
                if !IGNORE_ERRS.contains(&code) {
                    let text = String::from_utf8_lossy(&ascii);
                    println!("Code {code:x}: {text}");
                }
            }
            Message::Unknown(data) => {
                println!("Unknown message type 0x{0:x}: {data:#x?}", ipp_message.message_type);
            }
            Message::DESFireRead(data) => {
                println!("DESFireRead: {data:#x?}");
            }
            Message::DESFireCardRemoved() => {println!("DESFireCardRemoved")}
            Message::EMVStatus(_) => {}
            Message::ProxCardFunctionReply(card_type) => {println!("PCFR Type: 0x{card_type:x}");}

            Message::UltralightReply(reply) => {
                match reply {
                    UltralightSubcommandReply::CardEnteredField (_uid) => {
                        println!("Card entered field");
                        // let cmd1 = Message::UltralightCommand(2, vec![0, 135]).to_ipp();
                        port.write_all(
                            &Message::UltralightCommand(
                                UltralightSubcommand::Read { start_page: 0, num_pages: 135 }
                            ).to_ipp()
                        ).unwrap();
                    }
                    UltralightSubcommandReply::CardLeftField => {
                        println!("Card left field");
                    }
                    UltralightSubcommandReply::ReadReply(payload) => {
                        for rec in NTAG::from(payload).unwrap().records {
                            match rec {
                                NTAGPayload::Text { lang, txt } => {
                                    println!("({}) Text: {}", String::from_utf8_lossy(&lang), String::from_utf8_lossy(&txt));
                                }
                                NTAGPayload::URI { txt } => {
                                    println!("URI: {}", String::from_utf8_lossy(&txt));
                                }
                                NTAGPayload::Unknown(data) => {
                                    println!("Unknown: {:#x?}", data);
                                }
                            }
                        }
                    }
                    _ => {
                        println!("UltralightReply CMD: {reply:#?}");
                    }
                }
            }
            _ => {
                println!("Other: {ipp_message:#x?}");
            }
        }

        // Send keep-alive every once in a while
        if clk.elapsed().unwrap().as_secs() >= 5 {
            // port.write_all(&status_message()).unwrap();
            port.write_all(&Message::Status.to_ipp()).unwrap();
            clk = SystemTime::now();
        }
    }
}

pub fn __debug_build_message(seq:u8, msgtype:u8, len:u16, contents:Vec<u8>) -> Vec<u8> {
    let mut msg = vec![0xBC, seq, 0x0, msgtype, (len >> 8 as u8).try_into().unwrap(), len as u8, ];
    msg.push(HDR_CRC.checksum(&msg));
    msg.extend_from_slice(&contents);
    if msgtype >= 0x80 {msg.extend_from_slice(&DATA_CRC.checksum(&contents).to_le_bytes())};
    return msg;
}