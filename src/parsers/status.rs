use anyhow::Result;

use crate::{
    bird::BirdStatus,
    parsers::parser::{Block, Parse},
};

impl Parse<Block> for BirdStatus {
    /// Parse the status output of bird response
    fn parse(lines: Block) -> Result<BirdStatus> {
        let mut status = BirdStatus::default();
        for line in lines {
            parse_line(&mut status, &line)?;
        }
        Ok(status)
    }
}

fn parse_version(status: &mut BirdStatus, line: &str) {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let version = tokens[2].to_string();
    status.version = version;
}

fn parse_router_id(status: &mut BirdStatus, line: &str) {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let router_id = tokens[tokens.len() - 1].to_string();
    status.router_id = router_id;
}

fn parse_server_time(status: &mut BirdStatus, line: &str) {
    let s = line.strip_prefix(" Current server time is ").unwrap_or("");
    status.current_server = s.to_string(); // unparsed.
}

fn parse_last_reboot(status: &mut BirdStatus, line: &str) {
    let s = line.strip_prefix(" Last reboot on ").unwrap_or("");
    status.last_reboot = s.to_string(); // unparsed.
}

fn parse_last_reconfig(status: &mut BirdStatus, line: &str) {
    let s = line.strip_prefix(" Last reconfiguration on ").unwrap_or("");
    status.last_reconfig = s.to_string(); // unparsed.
}

fn parse_message(status: &mut BirdStatus, line: &str) {
    let message = line.strip_prefix("0013 ").unwrap_or("");
    status.message = message.to_string();
}

fn parse_line(status: &mut BirdStatus, line: &str) -> Result<()> {
    if line.starts_with("0001 ") {
        parse_version(status, line);
    } else if line.starts_with("1011-") {
        parse_router_id(status, line);
    } else if line.starts_with(" Current server time") {
        parse_server_time(status, line);
    } else if line.starts_with(" Last reboot") {
        parse_last_reboot(status, line);
    } else if line.starts_with(" Last reconfiguration") {
        parse_last_reconfig(status, line);
    } else if line.starts_with("0013 ") {
        parse_message(status, line);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    #[test]
    fn test_parse() {
        let file = File::open("tests/birdc/show-status").unwrap();
        let reader = BufReader::new(file);
        let block =
            reader.lines().map(|l| l.unwrap()).collect::<Vec<String>>();
        let status = BirdStatus::parse(block).unwrap();
        assert_eq!(status.version, "2.0.10");
        assert_eq!(status.router_id, "111.111.111.111");
        assert_eq!(status.current_server, "2023-05-10 14:27:32");
        assert_eq!(status.last_reboot, "2023-05-10 11:34:49");
        assert_eq!(status.last_reconfig, "2023-05-10 11:34:49");
        assert_eq!(status.message, "Daemon is up and running");
    }
}
