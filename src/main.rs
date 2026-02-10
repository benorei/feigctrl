#![warn(clippy::pedantic)]

use binrw::BinRead;
use std::io::{Cursor, Read};

use crate::ipp::IppMessage;

mod ipp;

fn main() {
    let mut port = serialport::new("/dev/ttyS3", 115_200)
        .timeout(std::time::Duration::MAX)
        .open()
        .expect("Failed to open serial port");

    // Max payload size + data CRC size + header size w/ one byte of extra padding
    let mut current_message = vec![0u8; 65536 + 4 + 8].into_boxed_slice();

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

        println!("{ipp_message:#x?}");
    }
}
