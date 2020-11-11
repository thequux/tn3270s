use crate::tn3270::stream::{ExtendedFieldAttribute, AID, WriteCommand, WriteCommandCode, WCC, WriteOrder, BufferAddressCalculator, FieldAttribute, StreamFormatError, IncomingRecord};
use crate::tn3270::Session;
use snafu::{Snafu, ResultExt};

#[derive(Copy, Clone, Debug)]
pub struct Address {
    pub row: u16,
    pub col: u16,
}

pub enum FieldData<'a> {
    RO(&'a str),
    RW(&'a mut String),
}

impl<'a> AsRef<str> for FieldData<'a> {
    fn as_ref(&self) -> &str {
        match self {
            FieldData::RO(data) => *data,
            FieldData::RW(data) => &**data,
        }
    }
}

pub struct  Field<'a> {
    pub address: Address,
    pub attrs: Vec<ExtendedFieldAttribute>,
    pub data: FieldData<'a>,
}

impl Field<'static> {
    pub fn at(row: u16, col: u16) -> Self {
        Self {
            address: Address {row, col},
            attrs: vec![],
            data: FieldData::RO(""),
        }
    }

}
impl<'a> Field<'a> {

    pub fn ro_text<'b>(self, text: &'b str) -> Field<'b> {
        Field {
            address: self.address,
            attrs: self.attrs,
            data: FieldData::RO(text),
        }
    }

    pub fn rw_text<'b>(self, text: &'b mut String) -> Field<'b> {
        Field {
            address: self.address,
            attrs: self.attrs,
            data: FieldData::RW(text),
        }
    }

    pub fn with_attr(mut self, attr: ExtendedFieldAttribute) -> Self {
        self.attrs.push(attr);
        self
    }
}

pub struct Screen<'a> {
    pub fields: Vec<Field<'a>>,
}

pub struct Response {
    pub address: Address,
    pub aid: AID,
}

#[derive(Snafu, Debug)]
pub enum ScreenError {
    IoError { context: &'static str, source: std::io::Error },
    StreamError { source: StreamFormatError },
}

impl<'a> Screen<'a> {
    pub fn present(&mut self, session: &mut Session) -> Result<Response, ScreenError> {
        let acalc = BufferAddressCalculator {
            width: 80,
            height: 24,
        };

        {
            let command = WriteCommand {
                command: WriteCommandCode::EraseWrite,
                wcc: WCC::RESET_MDT | WCC::KBD_RESTORE,
                orders: self.fields.iter()
                    .flat_map(|field| {
                        use std::iter::*;
                        let Address { row, col } = field.address;
                        let bufaddr = acalc.encode_address(row, col);

                        let ro = if let FieldData::RO(_) = field.data { true } else { false };

                        let mut field_attr = field.attrs.clone();
                        let mut have_fa = false;
                        for attr in field_attr.iter_mut() {
                            if let ExtendedFieldAttribute::FieldAttribute(attr) = attr {
                                attr.set(FieldAttribute::PROTECTED, ro);
                                have_fa = true;
                            }
                        }
                        if !have_fa {
                            field_attr.insert(0, ExtendedFieldAttribute::FieldAttribute(if ro {
                                FieldAttribute::PROTECTED
                            } else {
                                FieldAttribute::NONE
                            }));
                        }

                        vec![
                            WriteOrder::SetBufferAddress(bufaddr),
                            WriteOrder::StartFieldExtended(field_attr),
                            WriteOrder::SendText(field.data.as_ref().to_owned()) ,
                            WriteOrder::StartField(FieldAttribute::PROTECTED),
                        ].into_iter()
                    })
                    .collect()
            };
            // eprintln!("Sending command: {:#?}", &command);
            session.send_record(&command).context(IoError { context: "Failed to send screen" })?;
        }

        let response = session.receive_record(None)
            .context(IoError { context: "Failed to read response" })?
            .unwrap(); // We can't get a None if we don't have a timeout

        let incoming = IncomingRecord::parse_record(response.as_slice())
            .context(StreamError)?;

        // eprintln!("Received: {:?}", incoming);

        let mut incoming_addr = !0;
        for order in incoming.orders {
            match order {
                WriteOrder::SetBufferAddress(addr) => {
                    incoming_addr = addr;
                }
                WriteOrder::SendText(text) => {
                    // TODO: Handle text that comes as multiple orders
                    for field in self.fields.iter_mut() {
                        if acalc.encode_address(field.address.row, field.address.col) == incoming_addr - 1 {
                            if let FieldData::RW(ref mut data) = field.data {
                                **data = text.clone();
                            }
                        }
                    }
                },
                _ => {},
            }
        }

        let (row, col) = acalc.decode_address(incoming.addr);

        Ok(Response {
            address: Address{ row, col },
            aid: incoming.aid,
        })
    }
}