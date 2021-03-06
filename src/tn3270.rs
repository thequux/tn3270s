use libtelnet_rs::{
    Parser,
    events::*,
    telnet::{
        op_option as tn_opt,
        op_command as tn_cmd,
    }
};
use std::net::TcpStream;
use std::io::{Write, Read};
use std::time::Duration;
use std::collections::VecDeque;

pub mod stream;
pub mod screen;

pub struct Session {

    parser: Parser,

    stream: std::net::TcpStream,

    term_type: Option<Vec<u8>>,
    is_eor: bool,
    is_bin: bool,

    incoming_records: VecDeque<Vec<u8>>,
    cur_record: Vec<u8>,
}

type Error = std::io::Error;

impl Session {
    pub fn new(stream: TcpStream) -> Result<Self, Error> {
        let mut session = Session {
            parser: Parser::new(),
            incoming_records: VecDeque::new(),
            stream,
            term_type: None,
            is_bin: false,
            is_eor: false,
            cur_record: Vec::new(),
        };

        session.parser.options.support(tn_opt::EOR);
        session.parser.options.support_remote(tn_opt::TTYPE);
        session.parser.options.support(tn_opt::TTYPE);
        session.parser.options.support(tn_opt::BINARY);

        // eprintln!("Negotiating...");
        session.negotiate()?;
        // eprintln!("Negotiation complete.");
        Ok(session)
    }

    fn option_state(&self, opt: u8) -> bool {
        let opt = self.parser.options.get_option(opt);
        opt.local_state && opt.remote_state
    }

    fn process_events(&mut self, mut events: Vec<TelnetEvents>) -> Result<(), Error> {
        let mut extra_events = Vec::new();
        let mut sendbuf = Vec::new();
        while !events.is_empty() || !extra_events.is_empty() {
            events.append(&mut extra_events);
            extra_events.truncate(0);
            for mut event in events.drain(..) {
                match event {
                    TelnetEvents::DataSend(ref mut data) => sendbuf.append(data),
                    TelnetEvents::DataReceive(ref mut data) => self.cur_record.append(data),
                    TelnetEvents::IAC(TelnetIAC { command: tn_cmd::EOR }) =>
                        self.incoming_records.push_back(std::mem::replace(&mut self.cur_record, Vec::new())),
                    TelnetEvents::IAC(iac) => eprintln!("Unknown IAC {}", iac.command),
                    TelnetEvents::Negotiation(TelnetNegotiation { command: tn_cmd::WILL, option: tn_opt::TTYPE }) => {
                        // eprintln!("WILL ttype");
                        let sub = self.parser.subnegotiation(tn_opt::TTYPE, vec![1]);
                        if let Some(event) = sub {
                            // eprintln!("Sending subnegotiation");
                            extra_events.push(event);
                        } else {
                            // eprintln!("Didn't do subnegotiation");
                        }

                    }
                    TelnetEvents::Negotiation(TelnetNegotiation { command, option }) => {
                        // eprintln!("Negotiate: {}/{}", command, option);
                        self.is_eor = self.option_state(tn_opt::EOR);
                        self.is_bin = self.option_state(tn_opt::BINARY);
                    }
                    TelnetEvents::Subnegotiation(TelnetSubnegotiation { option: tn_opt::TTYPE, buffer }) => {
                        if buffer[0] == 0 {
                            self.term_type = Some(buffer[1..].to_vec());

                            // If the terminal type is correct, we also need to negotiate EOR and BINARY
                            extra_events.extend(
                                [
                                    self.parser._will(tn_opt::EOR),
                                    self.parser._do(tn_opt::EOR),
                                    self.parser._will(tn_opt::BINARY),
                                    self.parser._do(tn_opt::BINARY),
                                ].iter_mut()
                                    .flat_map(Option::take)
                            )
                        }
                    }
                    TelnetEvents::Subnegotiation(_) => {},
                    TelnetEvents::DecompressImmediate(_) => unimplemented!("We don't support MCCP"),
                }
            }
        }

        // eprintln!("Sending: {:?}", &sendbuf);
        self.stream.write_all(sendbuf.as_slice())?;
        Ok(())
    }

    fn is_ready(&self) -> bool {
        self.term_type.is_some() && self.is_bin && self.is_eor
    }

    fn negotiate(&mut self) -> Result<bool, std::io::Error> {
        let mut initial_negotiation = vec![];
        initial_negotiation.extend(self.parser._do(tn_opt::TTYPE));
        initial_negotiation.extend(self.parser._will(tn_opt::TTYPE));

        self.process_events(initial_negotiation)?;

        // Large enough for a TCP packet
        let mut idata = Vec::with_capacity(2000);
        idata.resize(idata.capacity(), 0);

        // Make sure that negotiation completes quickly
        self.stream.set_read_timeout(Some(Duration::from_secs(5)))?;

        while !self.is_ready() {
            let len = self.stream.read(&mut idata[..])?;
            if len == 0 {
                return Ok(false)
            }
            let events = self.parser.receive(&idata[..len]);
            // eprintln!("Received events: {:#?}", &events);
            self.process_events(events)?;
        }

        self.stream.set_read_timeout(None)?;
        Ok(true)

    }

    pub fn send_record(&mut self, record: impl Into<Vec<u8>>) -> std::io::Result<()> {

        let mut send_data = Parser::escape_iac(record.into());
        send_data.extend_from_slice(&[libtelnet_rs::telnet::op_command::IAC, libtelnet_rs::telnet::op_command::EOR]);
        self.stream.write_all(send_data.as_slice())
    }


    pub fn receive_record(&mut self, timeout: Option<Duration>) -> std::io::Result<Option<Vec<u8>>> {
        if !self.incoming_records.is_empty() {
            return Ok(self.incoming_records.pop_front());
        }

        self.stream.set_read_timeout(timeout)?;
        let mut buf = vec![0; 1024];
        let mut len = self.stream.read(buf.as_mut_slice())?;
        if len != 0 {
            self.stream.set_nonblocking(true)?;
            while len != 0 {
                let events = self.parser.receive(&buf[..len]);
                self.process_events(events)?;
                len = match self.stream.read(buf.as_mut_slice()) {
                    Ok(len) => len,
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => 0,
                    Err(err) => return Err(err),
                };
            }
            self.stream.set_nonblocking(false)?;
        }

        self.stream.set_read_timeout(None)?;
        Ok(self.incoming_records.pop_front())
    }
}

