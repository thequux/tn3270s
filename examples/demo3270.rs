use structopt::StructOpt;
use std::time::Duration;

use tn3270s::tn3270;
use tn3270s::tn3270::stream::WriteOrder::SetBufferAddress;
use tn3270s::tn3270::screen::{Screen, Field, Address, FieldData};
use tn3270s::tn3270::stream::{ExtendedFieldAttribute, FieldAttribute};

#[derive(StructOpt)]
pub struct Cli {
    #[structopt(short="h", long = "host", default_value="::1")]
    host: String,
    #[structopt(short="p", long = "port", default_value="2101")]
    port: u16,

}

//      _~^~^~_
//  \) /  o o  \ (/
//    '_   ¬   _'
//    / '-----' \
//  1234567890123456

static rust_logo: [&'static str; 4] = [
  r#"     _~^~^~_     "#,
  r#" \) /  o o  \ (/ "#,
  r#"   '_   ¬   _'   "#,
  r#"   / '-----' \   "#,
];

fn intro_screen(session: &mut tn3270::Session) -> anyhow::Result<()> {
    use tn3270::stream::*;
    let bufsz = BufferAddressCalculator { width: 80, height: 24 };
    let mut record = WriteCommand {
        command: WriteCommandCode::Write,
        wcc: WCC::RESET | WCC::KBD_RESTORE | WCC::RESET_MDT,
        orders: vec![
            WriteOrder::SetBufferAddress(0),
            WriteOrder::EraseUnprotectedToAddress(bufsz.last_address()),
            WriteOrder::SetBufferAddress(bufsz.encode_address(1, 31)),
            WriteOrder::StartFieldExtended(vec![
                ExtendedFieldAttribute::FieldAttribute(FieldAttribute::PROTECTED),
                // ExtendedFieldAttribute::ForegroundColor(Color::Red),
            ]),
            WriteOrder::SendText("Hello from Rust!".into()),
            WriteOrder::SetBufferAddress(bufsz.encode_address(8, 21)),
            WriteOrder::StartField(FieldAttribute::INTENSE_SELECTOR_PEN_DETECTABLE),
            WriteOrder::SendText("        ".into()),
            WriteOrder::StartField(FieldAttribute::PROTECTED),
            WriteOrder::SetBufferAddress(bufsz.encode_address(8, 10)),
            WriteOrder::StartFieldExtended(vec![
                ExtendedFieldAttribute::FieldAttribute(FieldAttribute::PROTECTED),
                // ExtendedFieldAttribute::ForegroundColor(Color::Turquoise),
            ]),
            WriteOrder::SendText("Name:".into()),
        ],
    };

    for (i, line) in rust_logo.iter().enumerate() {
        record.orders.push(WriteOrder::SetBufferAddress(bufsz.encode_address(3+i as u16, 31)));
        record.orders.push(WriteOrder::StartFieldExtended(vec![
            ExtendedFieldAttribute::FieldAttribute(FieldAttribute::PROTECTED),
            ExtendedFieldAttribute::ForegroundColor(Color::Red),
        ]));
        record.orders.push(WriteOrder::SendText((*line).into()));
    }
    session.send_record(&record)?;
    session.send_record(&WriteCommand{
        command: WriteCommandCode::Write,
        wcc: WCC::RESET_MDT | WCC::KBD_RESTORE,
        orders: vec![],
    })?;

    let record = session.receive_record(None)?;
    if let Some(record) = record {
        eprintln!("Incoming record: {:?}", hex::encode(&record));
        eprintln!("Decoded: {:#?}", IncomingRecord::parse_record(record.as_slice()))
    } else {
        eprintln!("No record");
    }
    Ok(())
}


fn hlapi_demo(session: &mut tn3270::Session) -> anyhow::Result<()> {
    let mut name = "        ".to_string();
    let mut passwd = "        ".to_string();

    let result = Screen {
        fields: vec![
            Field::at(1, 32).ro_text("Please enter your data"),
            Field::at(3, 10).ro_text("Name: "),
            Field::at(3, 20).rw_text(&mut name),
            Field::at(4, 10).ro_text("Password: "),
            Field::at(4, 20).rw_text(&mut passwd)
                .with_attr(ExtendedFieldAttribute::FieldAttribute(FieldAttribute::NON_DISPLAY)),
        ],
    }.present(&mut *session)?;

    let aid = format!("{:?}", result.aid);
    Screen {
        fields: vec![
          Field::at(1, 32).ro_text("Your data"),
          Field::at(3, 10).ro_text("Name: "),
          Field::at(3, 20).ro_text(name.as_str()),
          Field::at(4, 10).ro_text("Password: "),
          Field::at(4, 20).ro_text(passwd.as_str()),
            Field::at(5, 10).ro_text("You pressed: "),
            Field::at(5, 25).ro_text(aid.as_str()),
            Field::at(23, 32).ro_text("Press ENTER to exit"),
        ],
    }.present(&mut *session)?;

    Ok(())
}

fn run(mut session: tn3270::Session) -> anyhow::Result<()> {
    intro_screen(&mut session);
    hlapi_demo(&mut session);

    // std::thread::sleep(Duration::from_secs(50));
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let options: Cli = Cli::from_args();
    let server = std::net::TcpListener::bind((options.host.as_str(), options.port))?;

    for client in server.incoming() {
        let client = client?;
        std::thread::spawn(move || {
            let session = match tn3270::Session::new(client) {
                Ok(session) => session,
                Err(err) => {
                    eprintln!("Error accepting session: {}", err);
                    return;
                }
            };

            if let Err(err) = run(session) {
                eprintln!("Error in session: {}", err);
            }

        });
    }
    println!("Hello, world!");

    Ok(())
}
