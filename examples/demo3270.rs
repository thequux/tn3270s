use structopt::StructOpt;
use std::time::Duration;

use tn3270s::tn3270;

#[derive(StructOpt)]
pub struct Cli {
    #[structopt(short="h", long = "host", default_value="[::1]")]
    host: String,
    #[structopt(short="p", long = "port", default_value="2101")]
    port: u16,

}

fn run(mut session: tn3270::Session) -> anyhow::Result<()> {
    use tn3270::stream::*;
    let mut record = WriteCommand::new(WriteCommandCode::Write, WCC::RESET);
    let bufsz = BufferAddressCalculator { width: 80, height: 24 };
    record.set_buffer_address(0)
        .erase_unprotected_to_address(bufsz.encode_address(79, 23))
        .set_buffer_address(bufsz.encode_address(31,1))
        .set_attribute(ExtendedFieldAttribute::ForegroundColor(Color::Red))
        .send_text("Hello from Rust!");
    session.send_record(record)?;

    std::thread::sleep(Duration::from_secs(60));
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
