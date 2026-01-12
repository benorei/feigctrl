use binrw::binrw;

#[binrw]
#[derive(Debug)]
pub enum MessageType {
    #[br(magic = 0x2u8)]
    Version,
    #[br(magic = 0x3u8)]
    VersionReply,

    #[br(magic = 0x4u8)]
    Status,
    #[br(magic = 0x5u8)]
    StatusReply,

    #[br(magic = 0x7u8)]
    Heartbeat,
    #[br(magic = 0xFu8)]
    Startup,

    #[br(magic = 0xBEu8)]
    PICCRead,

    #[br(magic = 0xD1u8)]
    EMVTransactionSuccessUnk,

    Unknown(u8),
}

#[binrw]
#[brw(big, magic = 0xbcu8)]
#[derive(Debug)]
pub struct IppHeader {
    pub sequence_number: u8,
    pub unk: u8,

    // MSB of this (when set to 1) determines whether the message uses a data CRC or not
    pub message_type: MessageType,

    pub data_length: u16,
    pub crc: u8,
}

#[binrw]
#[brw(big)]
#[derive(Debug)]
pub struct IppMessage {
    pub header: IppHeader,

    #[br(count = header.data_length)]
    pub data: Vec<u8>,
}
