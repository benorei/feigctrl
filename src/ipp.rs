use binrw::{BinRead, BinResult, BinWrite, binrw};
use std::{io::Cursor};
use crc::{Crc, CRC_8_MAXIM_DOW, CRC_32_JAMCRC};

pub const HDR_CRC: Crc::<u8> = Crc::<u8>::new(&CRC_8_MAXIM_DOW);
pub const DATA_CRC: Crc::<u32> = Crc::<u32>::new(&CRC_32_JAMCRC);

#[binrw]
// #[br(import(message_type: u8, data_length: u16))]
#[derive(Debug)]
#[derive(Default)]
pub enum Message {
    Version,
    Status,
    Reset(u8),
    Lights(u32),
    LightsOn(#[bw(calc = 0x000000C0)] #[bw(big)] u32),
    LightsOff(#[bw(calc = 0x00000000)] #[bw(big)] u32),
    Buzzer(u16, u16),
    UltralightCommand(
        UltralightSubcommand,
    ),
    ProxCardFunction(
        #[bw(big)]
        u16, // Function
        
        #[bw(calc = 1)]
        u8, // always 1
        u8 // Set function
    ),
    #[default]
    Empty,
}

impl Message {
    fn bin(&self) -> Vec<u8> {
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
            Message::Version => {0x2}
            Message::Status => {0x4}
            Message::Reset(..) => {0x10}
            Message::Lights(..) => {0x20}
            Message::LightsOn() => {0x20}
            Message::LightsOff() => {0x20}
            Message::Buzzer(..) => {0x22}
            Message::UltralightCommand(..) => {0xD4}
            Message::ProxCardFunction(..) => {0xE4}
            Message::Empty => {0xFF}
        }
    }
}

#[binrw]
#[br(import(message_type: u8, data_length: u16))]
#[derive(Debug)]
pub enum MessageReply {
    // Version
    #[br(pre_assert(message_type == 0x3u8))]
    VersionReply(
        #[br(parse_with = parse_string)]
        #[br(args(data_length))]
        #[bw(ignore)] // Should never write this message
        String
    ),

    // Status
    // TODO: Remove unnecessary fields
    #[br(pre_assert(message_type == 0x5u8))]
    StatusReply(#[br(count = data_length)] Vec<u8>),

    // Heartbeat
    // TODO: Change vec to heartbeat #
    #[br(pre_assert(message_type == 0x7u8))]
    Heartbeat(#[br(count = data_length)] Vec<u8>),

    // Startup
    #[br(pre_assert(message_type == 0xFu8))]
    Startup(#[br(count = data_length)] Vec<u8>),

    // Reset
    #[br(pre_assert(message_type == 0x11u8))]
    ResetReply(#[br(count = data_length)] Vec<u8>),

    // DesFire
    #[br(pre_assert(message_type == 0xB9u8))]
    DESFireRead(#[br(count = data_length)] Vec<u8>),

    #[br(pre_assert(message_type == 0xBBu8))]
    DESFireCardRemoved(),

    #[br(pre_assert(message_type == 0xBEu8))]
    UnhandledCard(
        u16,
        #[br(count = data_length - 1)] Vec<u8>
    ),

    // EMV
    #[br(pre_assert(message_type == 0xD1u8))]
    EMVStatus(#[br(count = data_length)] Vec<u8>),

    // Ultralight
    #[br(pre_assert(message_type == 0xD5u8))]
    UltralightReply(
        #[br(temp)]
        #[bw(ignore)]  // Will never be used but required for compilation
        u8, // cmd echo
        #[br(temp)]
        #[bw(ignore)] // Will never be used but required for compilation
        u8, // status
        // Give data length !not including! subcmd u8
        #[br(args(self_0, self_1, data_length - 2))]
        UltralightSubcommandReply,
    ),

    // ProxCardFunction
    #[br(pre_assert(message_type == 0xE5u8))]
    ProxCardFunctionReply(
        #[br(pad_before = 1)]
        #[br(pad_after = 2)]
        u8
    ),

    // Log
    #[br(pre_assert(message_type == 0xEDu8))]
    Log(
        u8,
        #[br(parse_with = parse_string)]
        #[br(args(data_length - 1))]
        #[bw(ignore)] // Should never write this message
        String,
    ),

    Unknown(
        #[br(calc = message_type)] #[bw(ignore)] u8,
        #[br(count = data_length)] Vec<u8>
    ),
}

#[allow(unused)]
impl MessageReply {
    pub fn kind(&self) -> u8 {
        match self {
            MessageReply::VersionReply(..) => {0x3}
            MessageReply::StatusReply(..) => {0x5}
            MessageReply::Heartbeat(..) => {0x7}
            MessageReply::ResetReply(..) => {0x11}
            MessageReply::Startup(..) => {0xF}
            MessageReply::DESFireRead(..) => {0xB9}
            MessageReply::DESFireCardRemoved(..) => {0xBB}
            MessageReply::UnhandledCard(..) => {0xBE}
            MessageReply::EMVStatus(..) => {0xD1}
            MessageReply::UltralightReply(..) => {0xD5}
            MessageReply::ProxCardFunctionReply(..) => {0xE5}
            MessageReply::Log(..) => {0xED}
            MessageReply::Unknown(..) => {0x00}
        }
    }
}

// Write Only
#[binrw]
// #[br(import(payload_length: u16))]
#[derive(Debug)]
pub enum UltralightSubcommand {
    // payload_length !does not! include subcmd u8.
    #[bw(magic = 0x02u8)]
    // #[br(pre_assert(payload_length == 2))]
    Read {
        start_page: u8, num_pages: u8,
    },

    #[bw(magic = 0x03u8)]
    // #[br(pre_assert(payload_length == 5))]
    Write {
        page_addr: u8,
        data: [u8; 4],
    },

    #[bw(magic = 0x04u8)]
    // #[br(pre_assert(payload_length == 16))]
    AuthenticateRawKey ([u8; 16]),

    #[bw(magic = 0x05u8)]
    AuthenticateKeyIndex (u16),

    #[bw(magic = 0x06u8)]
    // #[br(pre_assert(payload_length == 18))]
    AuthenticateIndexAndKey (u16, [u8; 16]),

    #[bw(magic = 0x07u8)]
    // #[br(pre_assert(payload_length == 0))]
    Abort,

    // Invalid(u8, #[br(count = payload_length)] Vec<u8>),
}

// Read Only
#[binrw]
#[br(import(subcmd: u8, status_code: u8, payload_length: u16))]
#[derive(Debug)]
pub enum UltralightSubcommandReply {

    #[br(pre_assert(subcmd == 0x00u8 && status_code == 0x00u8))]
    CardEnteredField (
        #[br(temp)]
        #[bw(ignore)] // Never used but required for compilation
        u8,                                        // Unknown
        #[br(count = payload_length - 1)] Vec<u8>,    // UID
    ),

    #[br(pre_assert(subcmd == 0x01u8 && status_code == 0x00u8 && payload_length == 0))]
    CardLeftField,

    #[br(pre_assert(subcmd == 0x02u8 && status_code == 0x00u8))]
    // #[br(magic = 0x02u8)]
    ReadReply (
        #[br(count = payload_length)]
        Vec<u8>
    ),

    #[br(pre_assert(subcmd == 0x03u8 && status_code == 0x00u8))]
    // #[br(magic = 0x03u8)]
    #[br(pre_assert(payload_length == 0))]
    WriteReply,

    #[br(pre_assert(subcmd == 0x04u8 && status_code == 0x00u8))]
    // #[br(magic = 0x04u8)]
    #[br(pre_assert(payload_length == 0))]
    AuthenticateRawKeyReply,

    #[br(pre_assert(subcmd == 0x05u8 && status_code == 0x00u8))]
    // #[br(magic = 0x06u8)]
    #[br(pre_assert(payload_length == 0))]
    AuthenticateKeyIndexReply,

    #[br(pre_assert(subcmd == 0x06u8 && status_code == 0x00u8))]
    // #[br(magic = 0x06u8)]
    #[br(pre_assert(payload_length == 0))]
    AuthenticateIndexAndKeyReply,

    

    #[br(pre_assert(subcmd == 0x07u8 && status_code == 0x00u8))]
    // #[br(magic = 0x07u8)]
    #[br(pre_assert(payload_length == 0))]
    AbortReply,

    #[br(pre_assert(status_code == 0x01u8))]
    OperationError{
        #[br(calc = subcmd)] subcmd: u8,
        #[br(count = payload_length)] payload: Vec<u8>,
    },

    #[br(pre_assert(status_code == 0x02u8))]
    InvalidParameters{
        #[br(calc = subcmd)] subcmd: u8,
        #[br(count = payload_length)] payload: Vec<u8>,
    },

    UnknownReply{

        #[br(count = payload_length)] payload: Vec<u8>,
    },
}

#[binrw]
#[brw(big, magic = 0xbcu8)]
#[derive(Debug)]
pub struct IppMessage {
    pub sequence_number: u8,
    pub reply_to: u8,

    // MSB of this (when set to 1) determines whether the message uses a data CRC or not
    pub message_type: u8,

    pub data_length: u16,
    pub crc_header: u8,

    #[br(ignore)]
    pub data: Message,

    #[br(args(message_type, data_length))]
    #[bw(ignore)]
    pub reply_data: MessageReply,

    #[brw(if((message_type >> 7) > 0))]
    pub data_crc: u32,
}

// There is another version of this in ntag
#[binrw::parser(reader, endian)]
fn parse_string (strlen: u16) -> BinResult<String> {
    let mut v = Vec::<u8>::new();
    for _ in 0..strlen {
        v.push(<_>::read_options(reader, endian, ())?);
    }
    Ok(String::from_utf8_lossy(&v).into_owned())
}