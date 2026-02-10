use binrw::binrw;

#[binrw]
#[br(import(message_type: u8, data_length: u16))]
#[derive(Debug)]
pub enum Message {
    #[br(pre_assert(message_type == 0x2u8))]
    Version(#[br(count = data_length)] Vec<u8>),
    #[br(pre_assert(message_type == 0x3u8))]
    VersionReply(#[br(count = data_length)] Vec<u8>),

    #[br(pre_assert(message_type == 0x4u8))]
    Status(#[br(count = data_length)] Vec<u8>),
    #[br(pre_assert(message_type == 0x5u8))]
    StatusReply(#[br(count = data_length)] Vec<u8>),

    #[br(pre_assert(message_type == 0x7u8))]
    Heartbeat(#[br(count = data_length)] Vec<u8>),
    #[br(pre_assert(message_type == 0xFu8))]
    Startup(#[br(count = data_length)] Vec<u8>),

    // >=0x80 has crc
    #[br(pre_assert(message_type == 0xBEu8))]
    PICCRead(#[br(count = data_length)] Vec<u8>),

    #[br(pre_assert(message_type == 0xD1u8))]
    EMVTransactionSuccessUnk(#[br(count = data_length)] Vec<u8>),

    Unknown(#[br(count = data_length)] Vec<u8>),
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
