#![warn(clippy::pedantic)]

use std::time::{SystemTime};

use crate::ipp::{*, Message::*, MessageReply::*, UltralightSubcommand::*, UltralightSubcommandReply::*};
use crate::ntag::*;
use crate::feig::*;

mod ipp;
mod ntag;
mod feig;

const IGNORE_ERRS: [u8; 4] = [2, 3, 5, 0xFE];

// TODO
// - Decide if I want to support NTAG writes
// - Update NTAG record types to automatically parse text
// - Update response types to have nicer parsable results
// - Fix large records not being able to be read: see if there is another ntag/nfc parser

fn main() {
    let mut feig = Feig::new().expect("Failed to open serial port");
    feig.send(ProxCardFunction(10, 1)).unwrap();
    feig.send(Version).unwrap();


    let mut clk = SystemTime::now();
    let mut lights_on = false;

    loop {
        let msg = match feig.receive() {
            Err(_e) => {continue}
            Ok(m) => m
        };

        match msg {
            VersionReply(data) => {
                println!("VersionReply: {data:#x?}");
            }
            StatusReply(sr) => {
                if sr[0] != 0x0 || sr[1] != 0x0 {println!("StatusReply {:#x}, {:#x}", sr[0], sr[1])};
            }
            ResetReply(rr) => {
                println!("ResetReply {:#x}, {:#x}", rr[0], rr[1]);
            }
            Startup(_) => {println!("Startup");}
            UnhandledCard(card_type, _data) => {
                println!("Unhandled Card: Type {card_type} (0x{card_type:x})"); 
                feig.send(Buzzer(6, 1)).unwrap();
            }
            Heartbeat(_) => {
                let msg: Message;
                if lights_on {msg = LightsOff()} else {msg = LightsOn()}
                feig.send(msg).unwrap();
                lights_on = !lights_on;
            }
            Log(code, txt) => {
                if !IGNORE_ERRS.contains(&code) {
                    println!("Code {code:x}: {txt}");
                }
            }
            Unknown(kind, data) => {
                println!("Unknown message type 0x{0:x}: {data:#x?}", kind);
            }
            DESFireCardRemoved() => {println!("DESFireCardRemoved")}
            UltralightReply(reply) => {
                match reply {
                    CardEnteredField (_uid) => {
                        feig.send(UltralightCommand(
                            Read { start_page: 0, num_pages: 135 }
                        )).unwrap();
                    }
                    CardLeftField => {}
                    ReadReply(payload) => {
                        for rec in NTAG::from(payload).unwrap().records {
                            match rec {
                                NTAGPayload::Text { lang, txt } => {
                                    println!("({:}) Text: {txt}", lang.to_string());
                                }
                                NTAGPayload::URI { txt } => {
                                    println!("URI: {txt}");
                                }
                                NTAGPayload::Unknown(data) => {
                                    println!("Unknown: {:#x?}", data);
                                }
                            }
                        }
                    }
                    _ => {
                        println!("UltralightReply: {reply:?}");
                    }
                }
            }
            _ => {}
        }

        // Send keep-alive every once in a while
        if clk.elapsed().unwrap().as_secs() >= 5 {
            // port.write_all(&Status.to_ipp()).unwrap();
            feig.send(Status).unwrap();
            clk = SystemTime::now();
        }
    }
}