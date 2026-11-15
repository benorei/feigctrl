use binrw::{binrw, BinWrite};
use std::io::Cursor;
use crc::{Crc, CRC_8_MAXIM_DOW, CRC_32_JAMCRC};

pub const HDR_CRC: Crc::<u8> = Crc::<u8>::new(&CRC_8_MAXIM_DOW);
pub const DATA_CRC: Crc::<u32> = Crc::<u32>::new(&CRC_32_JAMCRC);

#[binrw]
#[br(import(message_type: u8, data_length: u16))]
#[derive(Debug)]
pub enum Message {
    // Version
    #[br(pre_assert(message_type == 0x2u8))]
    Version(#[br(count = data_length)] Vec<u8>),
    #[br(pre_assert(message_type == 0x3u8))]
    VersionReply(#[br(count = data_length)] Vec<u8>),

    // Status
    #[br(pre_assert(message_type == 0x4u8))]
    Status(#[br(count = data_length)] Vec<u8>),
    #[br(pre_assert(message_type == 0x5u8))]
    StatusReply(#[br(count = data_length)] Vec<u8>),

    // Heartbeat
    #[br(pre_assert(message_type == 0x7u8))]
    Heartbeat(#[br(count = data_length)] Vec<u8>),
    
    // Startup
    #[br(pre_assert(message_type == 0xFu8))]
    Startup(#[br(count = data_length)] Vec<u8>),

    // Reset
    #[br(pre_assert(message_type == 0x10u8))]
    Reset(u8),
    #[br(pre_assert(message_type == 0x11u8))]
    ResetReply(#[br(count = data_length)] Vec<u8>),

    // Buzzer
    #[br(pre_assert(message_type == 0x22u8 && data_length == 4))]
    Buzzer(u16, u16),

    // >=0x80 has crc
    #[br(pre_assert(message_type == 0xBEu8))]
    UnhandledCard(
        u16,
        #[br(count = data_length - 1)] Vec<u8>
    ),

    #[br(pre_assert(message_type == 0xEDu8))]
    Log(
        u8,
        #[br(count = data_length - 1)] Vec<u8>
    ),

    // EMV
    #[br(pre_assert(message_type == 0xD1u8))]
    EMVStatus(#[br(count = data_length)] Vec<u8>),

    // Ultralight
    #[br(pre_assert(message_type == 0xD4u8))]
    UltralightCommand(u8, #[br(count = data_length - 1)] Vec<u8>),
    #[br(pre_assert(message_type == 0xD5u8))]
    UltralightReply(
        u8, // cmd echo
        u8, // status
        #[br(count = data_length - 2)] Vec<u8> // payload
    ),

    #[br(pre_assert(message_type == 0xE5u8))]
    ProxCardFunctionReply(
        #[br(pad_before = 1)]
        u8,
        u16 // junk
    ),

    // DesFire
    #[br(pre_assert(message_type == 0xB9u8))]
    DESFireRead(#[br(count = data_length)] Vec<u8>),

    #[br(pre_assert(message_type == 0xBBu8))]
    DESFireCardRemoved(),

    Unknown(#[br(count = data_length)] Vec<u8>),
}

impl Message {
    pub fn bin(&self) -> Vec<u8> {
        let mut writer = Cursor::new(Vec::new());
        self.write_le(&mut writer).unwrap();
        return writer.into_inner();
    }

    pub fn to_ipp(&self) -> Vec<u8> {self.to_ipp_seq(0x0)}

    pub fn to_ipp_seq(&self, seq: u8) -> Vec<u8> {
        let bin = self.bin();
        let mut ipp: Vec<u8> = vec![
            0xBC,                   // Magic
            seq,                    // Seq
            0x0,                    // ReplyTo
            self.kind(),            // Message type
            (bin.len() >> 8 as u8).try_into().unwrap(), 
            bin.len() as u8,
        ];
        ipp.push(HDR_CRC.checksum(&ipp));
        ipp.extend_from_slice(&bin);
        if self.kind() >= 0x80 {ipp.extend_from_slice(&DATA_CRC.checksum(&bin).to_le_bytes())};
        return ipp;
    }

    pub fn kind(&self) -> u8 {
    match self {
        Message::Version(_) => {0x2}
        Message::VersionReply(_) => {0x3}
        Message::Status(_) => {0x4}
        Message::StatusReply(_) => {0x5}
        Message::Heartbeat(_) => {0x7}
        Message::Startup(_) => {0xF}
        Message::Reset(_) => {0x10}
        Message::ResetReply(_) => {0x11}
        Message::Buzzer(..) => {0x22}
        Message::DESFireRead(_) => {0xB9}
        Message::DESFireCardRemoved(..) => {0xBB}
        Message::UnhandledCard(..) => {0xBE}
        Message::EMVStatus(_) => {0xD1}
        Message::UltralightCommand(_, _) => {0xD4}
        Message::UltralightReply(..) => {0xD5}
        Message::ProxCardFunctionReply(_,_) => {0xE5}
        Message::Log(..) => {0xED}
        Message::Unknown(..) => {0x00}
    }
}
}

#[binrw]
#[brw(big, magic = 0xbcu8)]
#[derive(Debug)]
pub struct IppMessage {
    pub sequence_number: u8,
    pub unk: u8,

    // MSB of this (when set to 1) determines whether the message uses a data CRC or not
    pub message_type: u8,

    pub data_length: u16,
    pub crc_header: u8,

    #[br(args(message_type, data_length))]
    pub data: Message,

    #[brw(if((message_type >> 7) > 0))]
    pub data_crc: u32,
}