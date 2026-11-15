use std::io::{Cursor};

use binrw::{binrw, BinRead};

// https://www.nxp.com/docs/en/data-sheet/NTAG213_215_216.pdf

// Could read from NTAG by specifying function to parse with binrw
#[allow(unused)]
pub struct NTAG {
    pub header: NTAGHeader,
    pub records: Vec<NTAGPayload>
}

impl NTAG {
    // Create NTAG from UltralightReply payload. When sending an UltralightCommand, read from the top of the tag (page 0).
    pub fn from(ultralight_reply_payload: Vec<u8>) -> Result<NTAG, binrw::Error> {
        let mut cursor = Cursor::new(ultralight_reply_payload);
        let result = NTAGHeader::read_le(&mut cursor);
        match result {
            Err(e) => {return Err(e)}
            Ok(header) => {
                let mut records = Vec::<NTAGPayload>::new();
                let mut rem = header.data_len;
                while rem > 0 {
                    let result = NTAGRecord::read_le(&mut cursor);
                    match result {
                        Ok(r) => {
                            records.push(r.payload);
                            rem -= r.payload_len + 4;
                        }
                        Err(..) => {
                            break;
                        }
                    }
                }
                return Ok(NTAG {header: header, records: records});
            }
        }  
    }
}

#[binrw]
#[derive(Debug)]
pub struct NTAGHeader {
    pub serial: [u8; 9],
    internal: u8,
    lock: [u8; 2],
    pub capability_container: [u8; 4],
    unk0: u8,
    data_len: u8,

}

#[binrw]
#[derive(Debug)]
pub struct NTAGRecord {
    unk: u8,
    unk1: u8,           // usually 0x1
    pub payload_len: u8,
    pub record_type: u8,
    unk4: u8,

    #[br(args(record_type, payload_len - 1))]
    pub payload: NTAGPayload,
}

#[binrw]
#[br(import(record_type: u8, payload_len: u8))]
#[derive(Debug)]
pub enum NTAGPayload {
    #[br(pre_assert(record_type == 0x54u8))]
    Text{
        lang: [u8; 2],

        #[br(count = payload_len - 2)]
        txt: Vec<u8>,

    },

    #[br(pre_assert(record_type == 0x55u8))]
    URI{
        #[br(count = payload_len)]
        // #[br(pad_before = 1)]
        txt: Vec<u8>
    },

    Unknown(
        #[br(count = payload_len)]
        Vec<u8>
    ),

}