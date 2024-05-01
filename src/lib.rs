use std::{error, fmt, result, str::FromStr};

use bytebuffer::{ByteBuffer, ByteReader};


type Error = Box<dyn error::Error>;
type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Parsing error occured: {}", self.0)
    }
}

impl error::Error for ParseError {}


pub struct DnsQuery {
    pub header: DnsHeader,
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>
}

impl DnsQuery {
    pub fn new() -> DnsQuery {
        DnsQuery {
            header: DnsHeader::new(),
            questions: vec![],
            answers: vec![]
        }
    }

    pub fn from_buffer(buf: &[u8]) -> Result<DnsQuery> {
        let mut reader = ByteReader::from_bytes(&buf);

        let mut result = DnsQuery::new();
        result.header = DnsHeader::read_buf(&mut reader)?;

        for _ in 0..result.header.questions {
            result.questions.push(Question::read_buf(&mut reader)?)
        }

        Ok(result)
    }

    pub fn write_buf(&self, writer: &mut ByteBuffer) {
        self.header.write_buf(writer);
        for question in &self.questions {
            question.write_buf(writer);
        }
        for answer in &self.answers {
            answer.write_buf(writer);
        }
    }
}


pub struct DnsHeader {
    pub id: u16,
    pub flags: Flags,
    pub questions: u16,
    pub answers: u16,
    pub authorities: u16,
    pub additional: u16
}

impl DnsHeader {
    fn new() -> DnsHeader {
        DnsHeader {
            id: 0,
            flags: Flags::new(),
            questions: 0,
            answers: 0,
            authorities: 0,
            additional: 0
        }
    }

    fn read_buf(reader: &mut ByteReader) -> Result<DnsHeader> {
        let mut result = DnsHeader::new();
        result.id = reader.read_u16()?;
        result.flags = Flags::read_buf(reader)?;
        result.questions = reader.read_u16()?;
        result.answers = reader.read_u16()?;
        result.authorities = reader.read_u16()?;
        result.additional = reader.read_u16()?;
        Ok(result)
    }

    pub fn write_buf(&self, writer: &mut ByteBuffer) {
        writer.write_u16(self.id);
        self.flags.write_buf(writer);
        writer.write_u16(self.questions);
        writer.write_u16(self.answers);
        writer.write_u16(self.authorities);
        writer.write_u16(self.additional);
    }
}


pub struct Flags {
    pub qr: bool,
    pub opcode: u8,
    pub authorihative_answer: bool,
    pub truncate: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    // 3 bits zero
    pub response_code: u8
}

impl Flags {
    fn new() -> Flags {
        Flags {
            qr: false,
            opcode: 0,
            authorihative_answer: false,
            truncate: false,
            recursion_desired: false,
            recursion_available: false,
            response_code: 0
        }
    }

    fn read_buf(reader: &mut ByteReader) -> Result<Flags> {
        let mut result = Flags::new();
        result.qr = reader.read_bit()?;
        result.opcode = reader.read_bits(4)? as u8;
        result.authorihative_answer = reader.read_bit()?;
        result.truncate = reader.read_bit()?;
        result.recursion_desired = reader.read_bit()?;
        result.recursion_available = reader.read_bit()?;
        reader.read_bits(3)?;
        result.response_code = reader.read_bits(4)? as u8;
        Ok(result)
    }

    pub fn write_buf(&self, writer: &mut ByteBuffer) {
        writer.write_bit(self.qr);
        writer.write_bits(self.opcode.into(), 4);
        writer.write_bit(self.authorihative_answer);
        writer.write_bit(self.truncate);
        writer.write_bit(self.recursion_desired);
        writer.write_bit(self.recursion_available);
        writer.write_bits(0, 3);
        writer.write_bits(self.response_code.into(), 4);
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum QType {
    Unknown(u16),
    A, // 1
    CNAME, // 5
}

impl QType {
    fn from_u16(value: u16) -> Result<QType> {
        match value {
            1 => Ok(QType::A),
            5 => Ok(QType::CNAME),
            _ => Err(Box::new(ParseError("Unknown qtype".into())))
        }
    }

    fn to_u16(&self) -> u16 {
        match self {
            Self::A => 1,
            Self::CNAME => 5,
            Self::Unknown(v) => *v
        }
    }
}

impl FromStr for QType {
    type Err = ();

    fn from_str(input: &str) -> result::Result<QType, Self::Err> {
        match input {
            "A" => Ok(QType::A),
            "CNAME" => Ok(QType::CNAME),
            _ => Err(())
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum QClass {
    Unknown(u16),
    IN
}

impl QClass {
    fn from_u16(value: u16) -> Result<QClass> {
        match value {
            1 => Ok(QClass::IN),
            _ => Err(Box::new(ParseError("Unknown qclass".into())))
        }
    }

    fn to_u16(&self) -> u16 {
        match self {
            Self::IN => 1,
            Self::Unknown(v) => *v
        }
    }
}

impl FromStr for QClass {
    type Err = ();

    fn from_str(input: &str) -> result::Result<QClass, Self::Err> {
        match input {
            "IN" => Ok(QClass::IN),
            _ => Err(())
        }
    }
}


pub struct Question {
    pub qname: String,
    pub qtype: QType,
    pub qclass: QClass
}

fn none_if_zero(byte: u8) -> Option<u8> {
    match byte {
        0 => None,
        size => Some(size)
    }
}

impl Question {
    fn new() -> Question {
        Question {
            qname: String::new(),
            qtype: QType::Unknown(0),
            qclass: QClass::Unknown(0)
        }
    }

    fn read_buf(reader: &mut ByteReader) -> Result<Question> {
        let mut result: Question = Question::new();
        let mut tokens = vec![];
        while let Some(token_size) = none_if_zero(reader.read_u8()?) {
            let bytes = reader.read_bytes(token_size.into())?;
            let raw = bytes.as_slice();
            let s = match std::str::from_utf8(raw) {
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
            tokens.push(s.to_string());
        }
        result.qname = tokens.join(".");
        result.qtype = QType::from_u16(reader.read_u16()?)?;
        result.qclass = QClass::from_u16(reader.read_u16()?)?;
        Ok(result)
    }

    pub fn write_buf(&self, writer: &mut ByteBuffer) {
        for token in self.qname.split('.') {
            writer.write_u8(token.len() as u8);
            token.chars().for_each(|c| writer.write_u8(c as u8))
        }
        writer.write_u8(0);
        writer.write_u16(self.qtype.to_u16());
        writer.write_u16(self.qclass.to_u16());
    }
}


pub struct Answer {
    pub name: String,
    pub qtype: QType,
    pub qclass: QClass,
    pub ttl: u32,
    pub length: u16,
    pub data: Vec<u8>
}

impl Answer {
    pub fn write_buf(&self, writer: &mut ByteBuffer) {
        
        for token in self.name.split('.') {
            writer.write_u8(token.len() as u8);
            token.chars().for_each(|c| writer.write_u8(c as u8))
        }
        writer.write_u8(0);
        writer.write_u16(self.qtype.to_u16());
        writer.write_u16(self.qclass.to_u16());
        writer.write_u32(self.ttl);
        writer.write_u16(self.length);
        for c in &self.data {
            writer.write_u8(*c);
        }
    }
}

pub struct DnsRecord {
    pub qname: String,
    pub qclass: QClass,
    pub qtype: QType,
    entry: String
}

impl DnsRecord {
    pub fn new(qname: &str, qclass: &str, qtype: &str, entry: &str) -> DnsRecord {
        DnsRecord {
            qname: qname.to_string(),
            qclass: match QClass::from_str(qclass) {
                Ok(qclass) => qclass,
                Err(err) => panic!("Unknown qclass: {err:?}")
            },
            qtype: match QType::from_str(qtype) {
                Ok(qtype) => qtype,
                Err(err) => panic!("Unknown qtype: {err:?}")
            },
            entry: entry.to_string()
        }
    }
    
    pub fn length(&self) -> usize {
        if self.qtype == QType::A {
            4
        } else {
            self.entry.len() + 1
        }
    }

    pub fn data(&self) -> Result<Vec<u8>> {
        let mut result: Vec<u8> = vec![];
        if self.qtype == QType::A {
            for byte in self.entry.split('.') {
                result.push(byte.parse::<u8>()?)
            }
        } else {
            for token in self.entry.split('.') {
                result.push(token.len() as u8);
                token.chars().for_each(|c| result.push(c as u8))
            }
            result.push(0);
        }
        Ok(result)
    }
}
