use binrw::{BinRead, BinResult};
use serialport::{SerialPort};
use std::io::{Cursor, Result};

pub mod ipp;
pub mod ntag;
pub use crate::ipp::*;
pub use crate::ntag::*;

// TODO
// - Decide if I want to support NFC writes (probably not)
// - Update response types to have nicer parsable results
// - Fix large records not being able to be read: see if there is another ntag/nfc parser
// - Add more NTAG record types
// - Package to a crate and move script logic to a new Rust crate

pub struct Feig {
  port: Box<dyn SerialPort>,
  current_message: Box<[u8]>,
}

impl Feig {
  pub fn new() -> serialport::Result<Feig> {
    let port;
    match serialport::new("/dev/ttyS3", 115_200)
        .timeout(std::time::Duration::MAX)
        .open() {
      Ok(r) => {
        port = r;
      }
      Err(e) => {
        return Err(e);
      }
    }
    Ok(Feig {
      port: port,
      current_message: vec![0u8; 65536 + 4 + 8].into_boxed_slice(),
    })
  }

  pub fn send(&mut self, message: Message) -> Result<()> {
    return self.port.write_all(&message.to_ipp());
  }

  // Write binary message contents. Just the "message data" part of the IPP message.
  #[allow(unused)]
  pub fn write_binary(&mut self, seq: u8, msgtype: u8, len: u16, contents: &[u8]) -> Result<()> {
    let mut msg = vec![0xBC, seq, 0x0, msgtype, (len >> 8 as u8).try_into().unwrap(), len as u8, ];
    msg.push(HDR_CRC.checksum(&msg));
    msg.extend_from_slice(contents);
    if msgtype >= 0x80 {msg.extend_from_slice(&DATA_CRC.checksum(&contents).to_le_bytes())};
    return self.port.write_all(&msg);
  }

  // Blocking read.
  pub fn receive(&mut self) -> BinResult<MessageReply> {
    if let Err(e) = self.port.read(&mut self.current_message) {
      return BinResult::Err(e.into());
    } else {
      let mut out_cursor = Cursor::new(&self.current_message);
      match IppMessage::read(&mut out_cursor) {
        Err(e) => return Err(e),
        Ok(ipp) => return Ok(ipp.reply_data),
      };
    }
  }
}