use bitflags::bitflags;
use std::io::Write;
use std::convert::{TryFrom, TryInto};
use snafu::{Snafu, ensure};

#[derive(Clone, Debug, Snafu)]
pub enum StreamFormatError {
    #[snafu(display("Invalid AID: {:02x}", aid))]
    InvalidAID { aid: u8, },
    #[snafu(display("Record ended early"))]
    UnexpectedEOR,
    #[snafu(display("Invalid data"))]
    InvalidData,
}

const WCC_TRANS: [u8; 64] = [
    0x40, 0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7,
    0xC8, 0xC9, 0x4A, 0x4B, 0x4C, 0x4D, 0x4E, 0x4F,
    0x50, 0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7,
    0xD8, 0xD9, 0x5A, 0x5B, 0x5C, 0x5D, 0x5E, 0x5F,
    0x60, 0x61, 0xE2, 0xE3, 0xE4, 0xE5, 0xE6, 0xE7,
    0xE8, 0xE9, 0x6A, 0x6B, 0x6C, 0x6D, 0x6E, 0x6F,
    0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7,
    0xF8, 0xF9, 0x7A, 0x7B, 0x7C, 0x7D, 0x7E, 0x7F,
];

bitflags! {
    pub struct WCC: u8 {
        const UNUSED = 0x80;
        const RESET = 0x40;
        const PRINT_FLAG1 = 0x20;
        const PRINT_FLAG2 = 0x10;
        const START_PRINTER = 0x08;
        const SOUND_ALARM = 0x04;
        const KBD_RESTORE = 0x02;
        const RESET_MDT = 0x01;
    }
}

bitflags! {
    pub struct FieldAttribute: u8 {
        const HI_1 = 0x40;
        const HI_2 = 0x80;
        const PROTECTED = 0x20;
        const NUMERIC = 0x10;
        const NON_DISPLAY = 0x0C;
        const DISPLAY_SELECTOR_PEN_DETECTABLE = 0x04;
        const INTENSE_SELECTOR_PEN_DETECTABLE = 0x08;
        const MODIFIED = 0x01;
        const NONE = 0x00;
    }
}

fn make_ascii_translatable(val: u8) -> u8 {
    WCC_TRANS[(val & 0x3F) as usize]
}

impl WCC {
    pub fn to_ascii_compat(self) -> u8 {
        make_ascii_translatable(self.bits())
    }

    pub fn from_ascii_compat(value: u8) -> Self {
        Self::from_bits(value & 0x3F).unwrap()
    }
}

pub trait OutputRecord {
    type Response;

    fn write_to(&self, writer: &mut dyn Write) -> std::io::Result<()>;
}

#[derive(Debug, Clone)]
pub struct WriteCommand {
    pub command: WriteCommandCode,
    pub wcc: WCC,
    pub orders: Vec<WriteOrder>,
}

#[derive(Copy, Clone, Debug)]
pub enum WriteCommandCode {
    Write,
    EraseWrite,
    EraseWriteAlternate,
    EraseAllUnprotected,
    WriteStructuredField,
}

impl WriteCommandCode {
    pub fn to_command_code(self) -> u8 {
        match self {
            WriteCommandCode::Write => 0xF1,
            WriteCommandCode::EraseWrite => 0xF5,
            WriteCommandCode::EraseWriteAlternate => 0x7E,
            WriteCommandCode::EraseAllUnprotected => 0x6F,
            WriteCommandCode::WriteStructuredField => 0xF3,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum Color {
    Default,
    /// Black on displays, white on printers
    NeutralBG,
    Blue,
    Red,
    Pink,
    Green,
    Turquoise,
    Yellow,
    // White on displays, black on printers
    NeutralFG,
    Black,
    DeepBlue,
    Orange,
    Purple,
    PaleGreen,
    PaleTurquoise,
    Grey,
    White,
}

impl Into<u8> for Color {
    fn into(self) -> u8 {
        match self {
            Color::Default => 0x00,
            Color::NeutralBG => 0xF0,
            Color::Blue => 0xF1,
            Color::Red => 0xF2,
            Color::Pink => 0xF3,
            Color::Green => 0xF4,
            Color::Turquoise => 0xF5,
            Color::Yellow => 0xF6,
            Color::NeutralFG => 0xF7,
            Color::Black => 0xF8,
            Color::DeepBlue => 0xF9,
            Color::Orange => 0xFA,
            Color::Purple => 0xFB,
            Color::PaleGreen => 0xFC,
            Color::PaleTurquoise => 0xFD,
            Color::Grey => 0xFE,
            Color::White => 0xFF,
        }
    }
}

impl TryFrom<u8> for Color {
    type Error = StreamFormatError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Color::Default,
            0xF0 => Color::NeutralBG,
            0xF1 => Color::Blue,
            0xF2 => Color::Red,
            0xF3 => Color::Pink,
            0xF4 => Color::Green,
            0xF5 => Color::Turquoise,
            0xF6 => Color::Yellow,
            0xF7 => Color::NeutralFG,
            0xF8 => Color::Black,
            0xF9 => Color::DeepBlue,
            0xFA => Color::Orange,
            0xFB => Color::Purple,
            0xFC => Color::PaleGreen,
            0xFD => Color::PaleTurquoise,
            0xFE => Color::Grey,
            0xFF => Color::White,
            _ => return Err(StreamFormatError::InvalidData),
        })
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum Highlighting {
    Default = 0x00,
    Normal = 0xF0,
    Blink = 0xF1,
    Reverse = 0xF2,
    Underscore = 0xF4,
}

impl TryFrom<u8> for Highlighting {
    type Error = StreamFormatError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        Ok(match v {
            0x00 => Highlighting::Default,
            0xF0 => Highlighting::Normal,
            0xF1 => Highlighting::Blink,
            0xF2 => Highlighting::Reverse,
            0xF4 => Highlighting::Underscore,
            _ => return Err(StreamFormatError::InvalidData)
        })
    }
}

impl Into<u8> for Highlighting {
    fn into(self) -> u8 {
        self as u8
    }
}

bitflags! {
    pub struct FieldOutline: u8 {
        const NO_OUTLINE = 0;
        const UNDERLINE = 0b0001;
        const RIGHT = 0b0010;
        const OVERLINE = 0b0100;
        const LEFT = 0b1000;
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum Transparency {
    Default,
    Or,
    Xor,
    Opaque,
}

impl From<Transparency> for u8 {
    fn from(v: Transparency) -> u8 {
        match v {
            Transparency::Default => 0x00,
            Transparency::Or => 0xF0,
            Transparency::Xor => 0xF1,
            Transparency::Opaque => 0xF2,
        }
    }
}

impl TryFrom<u8> for Transparency {
    type Error = StreamFormatError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Transparency::Default,
            0xF0 => Transparency::Or,
            0xF1 => Transparency::Xor,
            0xF2 => Transparency::Opaque,
            _ => return Err(StreamFormatError::InvalidData)
        })
    }
}

bitflags! {
    pub struct FieldValidation: u8 {
        const MANDATORY_FILL = 0b100;
        const MANDATORY_ENTRY = 0b010;
        const TRIGGER = 0b001;
    }
}

#[derive(Copy, Clone, Debug, Hash)]
pub enum ExtendedFieldAttribute {
    AllAttributes,
    ExtendedHighlighting(Highlighting),
    ForegroundColor(Color),
    CharacterSet(u8),
    BackgroundColor(Color),
    Transparency(Transparency),
    FieldAttribute(FieldAttribute),
    FieldValidation(FieldValidation),
    FieldOutlining(FieldOutline),
}

impl TryFrom<&[u8]> for ExtendedFieldAttribute {
    type Error = StreamFormatError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        ensure!(value.len() == 2, UnexpectedEOR);
        Ok(match (value[0], value[1]) {
            (0x00, 0x00) => ExtendedFieldAttribute::AllAttributes,
            (0x00, _) => return Err(StreamFormatError::InvalidData),
            (0xC0, fa) => ExtendedFieldAttribute::FieldAttribute(FieldAttribute::from_bits(fa & 0x3F).ok_or(StreamFormatError::InvalidData)?),
            (0x41, v) => ExtendedFieldAttribute::ExtendedHighlighting(v.try_into()?),
            (0x45, v) => ExtendedFieldAttribute::BackgroundColor(v.try_into()?),
            (0x42, v) => ExtendedFieldAttribute::ForegroundColor(v.try_into()?),
            (0x43, v) => ExtendedFieldAttribute::CharacterSet(v),
            (0xC2, v) => ExtendedFieldAttribute::FieldOutlining(FieldOutline::from_bits(v).ok_or(StreamFormatError::InvalidData)?),
            (0x46, v) => ExtendedFieldAttribute::Transparency(v.try_into()?),
            (0xC1, v) => ExtendedFieldAttribute::FieldValidation(FieldValidation::from_bits(v).ok_or(StreamFormatError::InvalidData)?),
            _ => return Err(StreamFormatError::InvalidData),
        })
    }
}

impl ExtendedFieldAttribute {
    pub fn encoded(self) -> (u8, u8) {
        match self {
            ExtendedFieldAttribute::AllAttributes => (0x00,0x00),
            ExtendedFieldAttribute::FieldAttribute(fa) => (0xC0, make_ascii_translatable(fa.bits)),
            ExtendedFieldAttribute::ExtendedHighlighting(fa) => (0x41, fa.into()),
            ExtendedFieldAttribute::BackgroundColor(c) => (0x45, c.into()),
            ExtendedFieldAttribute::ForegroundColor(c) => (0x42, c.into()),
            ExtendedFieldAttribute::CharacterSet(cs) => (0x43, cs.into()),
            ExtendedFieldAttribute::FieldOutlining(fo) => (0xC2, fo.bits()),
            ExtendedFieldAttribute::Transparency(v) => (0x46, v.into()),
            ExtendedFieldAttribute::FieldValidation(v) => (0xC1, v.bits()),
        }
    }

    fn encode_into(&self, output: &mut Vec<u8>) {
        let (typ, val) = self.encoded();
        output.extend_from_slice(&[typ, val]);
    }

}

impl Into<ExtendedFieldAttribute> for &ExtendedFieldAttribute {
    fn into(self) -> ExtendedFieldAttribute {
        *self
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BufferAddressCalculator {
    pub width: u16,
    pub height: u16,
}

impl BufferAddressCalculator {
    pub fn encode_address(self, y: u16, x: u16) -> u16 {
        self.width * y + x
    }

    pub fn last_address(self) -> u16 {
        self.width * self.height - 1
    }

    pub fn decode_address(self, addr: u16) -> (u16, u16) {
        (addr / self.width, addr % self.width)
    }
}

#[derive(Clone, Debug)]
pub enum WriteOrder {
    StartField(FieldAttribute),
    /// The list of attributes MUST include a FieldAttribute
    StartFieldExtended(Vec<ExtendedFieldAttribute>),
    SetBufferAddress(u16),
    SetAttribute(ExtendedFieldAttribute),
    ModifyField(Vec<ExtendedFieldAttribute>),
    InsertCursor(u16),
    ProgramTab,
    RepeatToAddress(u16, char),
    EraseUnprotectedToAddress(u16),
    GraphicEscape(u8),
    SendText(String),
}

impl WriteOrder {

    pub fn serialize(&self, output: &mut Vec<u8>) {
        match self {
            WriteOrder::StartField(attr) => output.extend_from_slice(&[0x1D, attr.bits()]),
            WriteOrder::StartFieldExtended(attrs) => {
                output.extend_from_slice(&[0x29, attrs.len() as u8]);
                for attr in attrs {
                    attr.encode_into(&mut *output);
                }
            }
            WriteOrder::SetBufferAddress(addr) => output.extend_from_slice(&[0x11, (addr >> 8) as u8, (addr & 0xff) as u8]),
            WriteOrder::SetAttribute(attr) => {
                let (typ, val) = attr.encoded();
                output.extend_from_slice(&[0x28, typ, val]);
            }
            WriteOrder::ModifyField(attrs) => {
                output.extend_from_slice(&[0x2C, attrs.len() as u8]);
                for attr in attrs {
                    attr.encode_into(&mut* output);
                }
            }
            WriteOrder::InsertCursor(addr) => output.extend_from_slice(&[0x11, (addr >> 8) as u8, (addr & 0xff) as u8]),
            WriteOrder::ProgramTab => output.push(0x05),
            WriteOrder::RepeatToAddress(addr, ch) => {
                // TODO: COme up with a way to allow graphic escape here
                output.extend_from_slice(&[0x3C, (addr >> 8) as u8, (addr & 0xff) as u8, crate::encoding::cp037::ENCODE_TBL[*ch as usize]])
            }
            WriteOrder::EraseUnprotectedToAddress(addr) => {
                output.extend_from_slice(&[0x12, (addr >> 8) as u8, (addr & 0xff) as u8])
            }
            WriteOrder::GraphicEscape(ch) => output.extend_from_slice(&[0x08, *ch]),
            WriteOrder::SendText(text) => {
                output.extend(crate::encoding::to_cp037(text.chars()));
            }
        }
    }
}

impl WriteCommand {
    pub fn serialize(&self, output: &mut Vec<u8>) {
        output.push(self.command.to_command_code());
        output.push(self.wcc.to_ascii_compat());
        for order in self.orders.iter() {
            order.serialize(&mut *output);
        }
    }
}


impl Into<Vec<u8>> for &WriteCommand {
    fn into(self) -> Vec<u8> {
        let mut result = vec![];
        self.serialize(&mut result);
        result
    }
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum AID {
    NoAIDGenerated,
    NoAIDGeneratedPrinter,
    StructuredField,
    ReadPartition,
    TriggerAction,
    SysReq,
    PF1,
    PF2,
    PF3,
    PF4,
    PF5,
    PF6,
    PF7,
    PF8,
    PF9,
    PF10,
    PF11,
    PF12,
    PF13,
    PF14,
    PF15,
    PF16,
    PF17,
    PF18,
    PF19,
    PF20,
    PF21,
    PF22,
    PF23,
    PF24,
    PA1,
    PA2,
    PA3,
    Clear,
    ClearPartition,
    Enter,
    SelectorPenAttention,
    MagReaderOperatorID,
    MagReaderNumber,
}

impl From<AID> for u8 {
    fn from(aid: AID) -> u8 {
        use self::AID::*;
        match aid {
            NoAIDGenerated => 0x60,
            NoAIDGeneratedPrinter => 0xE8,
            StructuredField => 0x88,
            ReadPartition => 0x61,
            TriggerAction => 0x7f,
            SysReq => 0xf0,
            PF1 => 0xF1,
            PF2 => 0xF2,
            PF3 => 0xF3,
            PF4 => 0xF4,
            PF5 => 0xF5,
            PF6 => 0xF6,
            PF7 => 0xF7,
            PF8 => 0xF8,
            PF9 => 0xF9,
            PF10 => 0x7A,
            PF11 => 0x7B,
            PF12 => 0x7C,
            PF13 => 0xC1,
            PF14 => 0xC2,
            PF15 => 0xC3,
            PF16 => 0xC4,
            PF17 => 0xC5,
            PF18 => 0xC6,
            PF19 => 0xC7,
            PF20 => 0xC8,
            PF21 => 0xC9,
            PF22 => 0x4A,
            PF23 => 0x4B,
            PF24 => 0x4C,
            PA1 => 0x6C,
            PA2 => 0x6E,
            PA3 => 0x6B,
            Clear => 0x6D,
            ClearPartition => 0x6A,
            Enter => 0x7D,
            SelectorPenAttention => 0x7E,
            MagReaderOperatorID => 0xE6,
            MagReaderNumber => 0xE7,
        }
    }
}


impl TryFrom<u8> for AID {
    type Error = StreamFormatError;

    fn try_from(aid: u8) -> Result<Self, Self::Error> {
        use self::AID::*;
        Ok(match aid {
            0x60 => NoAIDGenerated,
            0xE8 => NoAIDGeneratedPrinter,
            0x88 => StructuredField,
            0x61 => ReadPartition,
            0x7f => TriggerAction,
            0xf0 => SysReq,
            0xF1 => PF1,
            0xF2 => PF2,
            0xF3 => PF3,
            0xF4 => PF4,
            0xF5 => PF5,
            0xF6 => PF6,
            0xF7 => PF7,
            0xF8 => PF8,
            0xF9 => PF9,
            0x7A => PF10,
            0x7B => PF11,
            0x7C => PF12,
            0xC1 => PF13,
            0xC2 => PF14,
            0xC3 => PF15,
            0xC4 => PF16,
            0xC5 => PF17,
            0xC6 => PF18,
            0xC7 => PF19,
            0xC8 => PF20,
            0xC9 => PF21,
            0x4A => PF22,
            0x4B => PF23,
            0x4C => PF24,
            0x6C => PA1,
            0x6E => PA2,
            0x6B => PA3,
            0x6D => Clear,
            0x6A => ClearPartition,
            0x7D => Enter,
            0x7E => SelectorPenAttention,
            0xE6 => MagReaderOperatorID,
            0xE7 => MagReaderNumber,
            _ => return Err(StreamFormatError::InvalidAID { aid }),
        })
    }
}

#[derive(Debug, Clone)]
pub struct IncomingRecord {
    pub aid: AID,
    pub addr: u16,
    pub orders: Vec<WriteOrder>,
}

fn parse_addr(encoded: &[u8]) -> Result<u16, StreamFormatError> {
    match encoded[0] >> 6 {
        0b00 => Ok(((encoded[0] as u16) << 8) + encoded[1] as u16),
        0b01 | 0b11 => {
            Ok((encoded[0] as u16 & 0x3F) << 6 | (encoded[1] as u16 & 0x3F))
        }
        _ => Err(StreamFormatError::InvalidData),
    }
}

impl IncomingRecord {
    pub fn parse_record(mut record: &[u8]) -> Result<Self, StreamFormatError> {
        if record.len() < 3 {
            return Err(StreamFormatError::UnexpectedEOR);
        }

        let aid = AID::try_from(record[0])?;
        // TODO: Handle AID 88 structured fields
        let addr = parse_addr(&record[1..3])?;

        let mut result = Self {
            aid,
            addr,
            orders: vec![]
        };

        record = &record[3..];

        while record.len() > 0 {
            match record[0] {
                0x1D => {
                    ensure!(record.len() >= 2, UnexpectedEOR);
                    result.orders.push(
                        WriteOrder::StartField(FieldAttribute::from_bits(record[1] & 0x3F)
                            .ok_or(StreamFormatError::InvalidData)?));
                    record = &record[2..];

                },
                0x29 => {
                    ensure!(record.len() >= 2, UnexpectedEOR);
                    let (header, body) = record.split_at(2);
                    let count = header[1] as usize;
                    ensure!(body.len() >= count * 2, UnexpectedEOR);
                    let (attrs, rest) = body.split_at(2 * count);
                    record = rest;

                    result.orders.push(
                        WriteOrder::StartFieldExtended(
                            attrs.chunks(2)
                            .map(ExtendedFieldAttribute::try_from)
                                .collect::<Result<Vec<ExtendedFieldAttribute>, StreamFormatError>>()?
                        )
                    )
                }
                0x11 => {
                    ensure!(record.len() >= 3, UnexpectedEOR);
                    result.orders.push(WriteOrder::SetBufferAddress(parse_addr(&record[1..3])?));
                    record = &record[3..];
                }
                0x28 => {
                    ensure!(record.len() >= 3, UnexpectedEOR);
                    result.orders.push(WriteOrder::SetAttribute(ExtendedFieldAttribute::try_from(&record[1..3])?));
                    record = &record[3..];
                }
                0x2C => {
                    ensure!(record.len() >= 2, UnexpectedEOR);
                    let (header, body) = record.split_at(2);
                    let count = header[1] as usize;
                    ensure!(body.len() >= count * 2, UnexpectedEOR);
                    let (attrs, rest) = body.split_at(2 * count);
                    record = rest;

                    result.orders.push(
                        WriteOrder::ModifyField(
                            attrs.chunks(2)
                                .map(ExtendedFieldAttribute::try_from)
                                .collect::<Result<Vec<ExtendedFieldAttribute>, StreamFormatError>>()?
                        )
                    )
                }
                0x13 => {
                    ensure!(record.len() >= 3, UnexpectedEOR);
                    result.orders.push(WriteOrder::InsertCursor(parse_addr(&record[1..3])?));
                    record = &record[3..];
                }
                0x05 => {
                    result.orders.push(WriteOrder::ProgramTab);
                    record = &record[1..];
                }
                0x3C => {
                    ensure!(record.len() >= 4, UnexpectedEOR);
                    // TODO: Handle graphic escape properly
                    result.orders.push(WriteOrder::RepeatToAddress(
                        parse_addr(&record[1..3])?,
                        crate::encoding::cp037::DECODE_TBL[record[4] as usize] as char,
                    ));
                    record = &record[4..]
                }
                0x12 => {
                    ensure!(record.len() >= 3, UnexpectedEOR);
                    result.orders.push(WriteOrder::EraseUnprotectedToAddress(parse_addr(&record[1..3])?));
                    record = &record[3..];
                }
                0x08 => {
                    ensure!(record.len() >= 2, UnexpectedEOR);
                    result.orders.push(WriteOrder::GraphicEscape(record[2]));
                    record = &record[2..];
                }
                0x40..=0xFF => {
                    let len = record.iter().position(|&v| v < 0x40).unwrap_or(record.len());
                    let data = record[..len]
                        .iter()
                        .map(|&v| crate::encoding::cp037::DECODE_TBL[v as usize] as char)
                        .collect();
                    result.orders.push(WriteOrder::SendText(data));
                    record = &record[len..];
                },
                _ => return Err(StreamFormatError::InvalidData)
            }
        }
        Ok(result)
    }
}