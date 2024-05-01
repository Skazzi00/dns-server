use std::{env, error, result, fs::File, io::{BufRead, BufReader}, net::UdpSocket};

use bytebuffer::ByteBuffer;
use dns_server::{Answer, DnsQuery, DnsRecord};
use rand::RngCore;

type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;


fn parse_file(file_path: &str) -> Vec<DnsRecord> {
    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) => panic!("Failed to open file: {err:?}")
    };
    let reader = BufReader::new(file);
    let mut records:  Vec<DnsRecord> = vec![];
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => panic!("Failed to read file: {err:?}")
        };
        let splitted: Vec<String> = line.split_whitespace().map(|s| s.to_string()).collect();
        if splitted.len() != 4 {
            continue;
        }
        let record = DnsRecord::new(
            &splitted[0],
            &splitted[1],
            &splitted[2],
            &splitted[3]
        );
        records.push(record);
    }
    records
}

fn handle(socket: &UdpSocket, records: &Vec<DnsRecord>) -> Result<()> {
    let mut buf = [0u8; 512];
    let (_, from) = socket.recv_from(&mut buf)?;
    let query = DnsQuery::from_buffer(&buf)?;
    
    let mut response = DnsQuery::new();
    for question in &query.questions {
        let matched_records: Vec<&DnsRecord> = records.iter()
            .filter(|record| record.qtype == question.qtype)
            .filter(|record| record.qclass == question.qclass)
            .filter(|record| record.qname == question.qname)
            .collect();
        if matched_records.is_empty() {
            continue;
        }
        let mut rng = rand::thread_rng();
        let index = rng.next_u32() % matched_records.len() as u32;
        let record = matched_records[index as usize];
        response.answers.push(Answer {
            name: record.qname.clone(),
            qclass: record.qclass.clone(),
            qtype: record.qtype.clone(),
            ttl: 60,
            length: record.length() as u16,
            data: record.data()?
        })
    }
    response.questions = query.questions;
    response.header = query.header;
    response.header.answers = response.answers.len() as u16;
    response.header.additional = 0;
    response.header.authorities = 0;
    response.header.flags.qr = true;
    response.header.flags.authorihative_answer = false;
    response.header.flags.truncate = false;
    response.header.flags.recursion_available = false;
    response.header.flags.response_code = if response.header.answers != response.header.questions {3} else { 0 };

    let mut buf = ByteBuffer::new();
    response.write_buf(&mut buf);

    socket.send_to(buf.as_bytes(), from)?;
    Ok(())
}

fn main(){
    let args: Vec<String> = env::args().collect();
    let file_path = &args[1];
    let records = parse_file(file_path);

    let socket = match UdpSocket::bind(("0.0.0.0", 5353)) {
        Ok(socket) => socket,
        Err(err) => panic!("Failed to open socket: {err:?}"),
    };

    loop {
        handle(&socket, &records).expect("fail to handle");
    }
}
