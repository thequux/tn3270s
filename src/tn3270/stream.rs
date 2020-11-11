use bitflags::bitflags;
use std::io::Write;

static WCC_TRANS: [u8; 64] = [
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
        const PROTECTED = 0x20;
        const NUMERIC = 0x10;
        const NON_DISPLAY = 0x0C;
        const DISPLAY_SELECTOR_PEN_DETECTABLE = 0x04;
        const INTENSE_SELECTOR_PEN_DETECTABLE = 0x08;
        const MODIFIED = 0x01;
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

pub struct WriteCommand {
    data: Vec<u8>,
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

#[derive(Copy, Clone, Debug, Hash)]
pub enum Highlighting {
    Default,
    Normal,
    Blink,
    Reverse,
    Underscore,
}

impl Into<u8> for Highlighting {
    fn into(self) -> u8 {
        match self {
            Highlighting::Default => 0x00,
            Highlighting::Normal => 0xF0,
            Highlighting::Blink => 0xF1,
            Highlighting::Reverse => 0xF2,
            Highlighting::Underscore => 0xF4,
        }
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
enum Transparency {
    Default,
    Or,
    Xor,
    Opaque,
}

impl Into<u8> for Transparency {
    fn into(self) -> u8 {
        match self {
            Transparency::Default => 0x00,
            Transparency::Or => 0xF0,
            Transparency::Xor => 0xF1,
            Transparency::Opaque => 0xF2,
        }
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
}

#[derive(Copy, Clone, Debug)]
pub struct BufferAddressCalculator {
    pub width: u16,
    pub height: u16,
}

impl BufferAddressCalculator {
    pub fn encode_address(self, x: u16, y: u16) -> u16 {
        self.width * y + x
    }
}

impl WriteCommand {
    pub fn new(command: WriteCommandCode, wcc: WCC) -> Self {
        WriteCommand {
            data: vec![command.to_command_code(), wcc.to_ascii_compat(),  ]
        }
    }

    pub fn start_field(&mut self, fa: FieldAttribute) -> &mut Self {
        self.data.push(0x1D);
        self.data.push(fa.bits());
        self
    }

    pub fn start_field_extended(&mut self, fa: FieldAttribute, attrs: impl IntoIterator<Item=ExtendedFieldAttribute>) -> &mut Self {
        self.data.push(0x29);
        let nattr_pos = self.data.len();
        self.data.push(0);
        let mut i = 1;
        self.data.push(0xC0);
        self.data.push(make_ascii_translatable(fa.bits()));

        for (typ, value) in attrs.into_iter().map(ExtendedFieldAttribute::encoded) {
            self.data.push(typ);
            self.data.push(value);
            i += 1;
        }
        self.data[nattr_pos] = i;
        self
    }

    pub fn set_buffer_address(&mut self, address: u16) -> &mut Self {
        self.data.push(0x11);
        self.data.push((address >> 8) as u8);
        self.data.push((address & 0xFF) as u8);
        return self;
    }

    pub fn set_attribute(&mut self, attr: ExtendedFieldAttribute) -> &mut Self {
        let (typ, val) = attr.encoded();
        self.data.extend_from_slice(&[0x28, typ, val]);
        self
    }

    pub fn modify_field(&mut self, attrs: impl IntoIterator<Item=ExtendedFieldAttribute>) -> &mut Self {
        self.data.push(0x2C);
        let nattr_pos = self.data.len();
        self.data.push(0);
        let mut i = 0;
        for (typ, value) in attrs.into_iter().map(ExtendedFieldAttribute::encoded) {
            self.data.push(typ);
            self.data.push(value);
            i += 1;
        }
        self.data[nattr_pos] = i;
        self
    }

    fn encode_address(&mut self, address: u16) {
        self.data.push((address >> 8) as u8);
        self.data.push((address & 0xFF) as u8);

    }

    pub fn insert_cursor(&mut self, address: u16) -> &mut Self {
        self.data.push(0x13);
        self.encode_address(address);
        return self;
    }

    pub fn program_tab(&mut self) -> &mut Self {
        self.data.push(0x05);
        self
    }

    // This must be followed by either a character or a graphic escape
    pub fn repeat_to_address(&mut self, address: u16) -> &mut Self {
        self.data.push(0x3C);
        self.encode_address(address);
        self
    }

    pub fn erase_unprotected_to_address(&mut self, address: u16) -> &mut Self {
        self.data.push(0x12);
        self.encode_address(address);
        self
    }

    pub fn graphic_escape(&mut self, charcode: u8) -> &mut Self {
        self.data.push(0x08);
        self.data.push(charcode);
        self
    }

    pub fn send_text(&mut self, data: &str) -> &mut Self {
        self.data.extend(crate::encoding::to_cp037(data.chars()));
        self
    }
}

impl AsRef<[u8]> for WriteCommand {
    fn as_ref(&self) -> &[u8] {
        self.data.as_slice()
    }
}

impl Into<Vec<u8>> for WriteCommand {
    fn into(self) -> Vec<u8> {
        self.data
    }
}