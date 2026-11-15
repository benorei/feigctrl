use binrw::{binrw};

#[binrw]
#[br(import(record_type: u8, payload_len: u8))]
#[derive(Debug)]
pub enum NTAGPayload {
    #[br(pre_assert(record_type == 0x54u8))]
    Text{
        lang: [u8; 2],

        #[br(count = payload_len - 3)]
        #[br(pad_after = 1)]
        txt: Vec<u8>,

    },
}

#[binrw]
#[derive(Debug)]
pub struct NTAGRecord {
    unk: u8,
    unk1: u8,           // usually 0x1
    pub payload_len: u8,
    pub record_type: u8,
    unk4: u8,

    #[br(args(record_type, payload_len))]
    pub payload: NTAGPayload,
}

// https://www.nxp.com/docs/en/data-sheet/NTAG213_215_216.pdf
#[binrw]
#[derive(Debug)]
pub struct NTAGHeader {
    pub serial: [u8; 9],
    internal: u8,
    lock: [u8; 2],
    pub capability_container: [u8; 4],
    unk0: u8,
    pub memsize: u8,

}