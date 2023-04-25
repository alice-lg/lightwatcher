use anyhow::Result;

use crate::parsers::parser::{Block, Parse};
use crate::state::Status;

impl Parse for Status {
    /// Parse the status output of bird response
    fn parse(lines: Block) -> Result<Status> {
        // Read first line and return bird version
        let line = lines.first().unwrap_or(&"".to_string()).to_owned();
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

    #[test]
    fn test_parse() {
        let input = vec!["0001 BIRD 1.6.3 ready.".to_string()];
        let status = Status::parse(input).unwrap();
    }
}
