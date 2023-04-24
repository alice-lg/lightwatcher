use anyhow::Result;
use std::io::{BufRead, BufReader, Read};

use crate::protocol::Status;

pub struct Parser;

impl Parser {
    pub fn parse<T: Read>(reader: BufReader<T>) -> Result<Status> {
        // Read first line and return bird version
        let mut lines = reader.lines();
        let line = lines.next().unwrap_or(Ok("".to_string()))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let version = tokens[2].to_string();
        let status = tokens[1..tokens.len()].join(" ");
        Ok(Status {
            bird_version: version,
            bird_status: status,
            ..Status::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse() {
        let input = "0001 BIRD 1.6.3 ready.";
        let reader = BufReader::new(Cursor::new(input));
        let status = Parser::parse(reader).unwrap();
        assert_eq!(status.bird_version, "1.6.3");
        assert_eq!(status.bird_status, "BIRD 1.6.3 ready.");
    }
}
