use std::io::{Cursor};

use binrw::{BinRead, BinResult, binrw};

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
                        Err(e) => {
                            return Err(e);
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
        lang: NTAGRecordLanguage,

        // #[br(count = payload_len - 2)]
        // txt: Vec<u8>,
        #[br(parse_with = parse_string)]
        #[br(args(payload_len - 2))]
        #[bw(ignore)] // Should never write this message
        txt: String,
    },

    #[br(pre_assert(record_type == 0x55u8))]
    URI{
        // #[br(count = payload_len)]
        // txt: Vec<u8>
        #[br(parse_with = parse_string)]
        #[br(args(payload_len - 2))]
        #[bw(ignore)] // Should never write this message
        txt: String,
    },

    Unknown(
        #[br(count = payload_len)]
        Vec<u8>
    ),

}

// There is another version of this in ipp
#[binrw::parser(reader, endian)]
fn parse_string (strlen: u8) -> BinResult<String> {
    let mut v = Vec::<u8>::new();
    for _ in 0..strlen {
        v.push(<_>::read_options(reader, endian, ())?);
    }
    Ok(String::from_utf8_lossy(&v).into_owned())
}

// #[binrw]
pub type NTAGRecordLanguage = [u8; 2];

pub trait NTAGRecordLanguageMethods {
    fn to_string(&self) -> String;
}

impl NTAGRecordLanguageMethods for NTAGRecordLanguage {
    fn to_string(&self) -> String {
        return String::from_utf8_lossy(self).into();
    }
}