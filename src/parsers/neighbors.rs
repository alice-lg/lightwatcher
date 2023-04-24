use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::io::{BufRead, BufReader, Read};

use rayon::prelude::*; // fix this

use crate::{
    parsers::{
        datetime,
        parser::{Block, BlockIterator, ParseError, Parser, Reader},
    },
    state::Neighbor,
};

lazy_static! {
    /// Regex: Neighbor header (protocol, state, uptime, ...)
    static ref RE_NEIGHBOR_HEADER: Regex = Regex::new(r"(?x)
        1002-(?P<protocol>\w+)   # protocol id
        \s+.*?\s+                # ignore this part
        (?P<state>\w+)           # state (up / down)
        \s+
        (?P<uptime>[\d\-:\s]+)    # since
        (\.\d+)?\s+?              # trailing time
        (?P<info>.*)$             # additional info
    ").unwrap();

    /// Regex for a Key: Value pair
    static ref RE_KEY_VALUE: Regex = Regex::new(r"(?x)
        .*?\s+
        (?P<key>[\s\w]+):
        \s+
        (?P<value>.+)
    ").unwrap();
}

/// Parser sections
#[derive(Debug, PartialEq, Clone)]
enum State {
    Start,
    Meta,
    BgpState,
    RouteChangeStats,
}

/// Implement reader for neighbor
impl Reader for Neighbor {
    type Item = Vec<Neighbor>;

    fn read<R: Read>(reader: BufReader<R>) -> Result<Self::Item> {
        let mut neighbors: Vec<Self> = vec![];
        let iterator = BlockIterator::new(reader, "1002-");
        for block in iterator {
            let neighbor = Neighbor::parse(block)?;
            if neighbor.id.is_empty() {
                continue;
            }
            neighbors.push(neighbor);
        }

        /*
        let blocks: Vec<Block> = BlockIterator::new(reader, "1002-").collect();
        let neighbors = blocks
            .par_iter()
            .map(|block| Neighbor::parse(block.clone()).unwrap())
            .filter(|neighbor| !neighbor.id.is_empty())
            .collect();
        */

        Ok(neighbors)
    }
}

/// Implement block parser for neighbor
impl Parser<Neighbor> for Neighbor {
    /// Parse a block of lines into a neighbor
    fn parse(block: Block) -> Result<Self> {
        let mut neighbor = Neighbor::default();

        // Parse lines in block
        let mut state = State::Start;
        for line in block.iter() {
            match parse_line(&mut neighbor, state, &line) {
                Ok(next_state) => state = next_state,
                Err(e) => {
                    println!("Error parsing line: {}, {}", line, e);
                    return Err(e);
                }
            }
        }

        Ok(neighbor)
    }
}

fn parse_line(mut neighbor: &mut Neighbor, state: State, line: &str) -> Result<State> {
    let state = match state {
        State::Start => parse_neighbor_header(&mut neighbor, line)?,
        State::Meta => parse_neighbor_meta(&mut neighbor, line)?,
        State::BgpState => parse_bgp_state(&mut neighbor, line)?,
        State::RouteChangeStats => parse_route_change_stats(&mut neighbor, line)?,
    };
    Ok(state)
}

/// Parse Neighbor Header (name, state, uptime) and return next state
fn parse_neighbor_header(neighbor: &mut Neighbor, line: &str) -> Result<State> {
    // Does line match neighbor header
    if !line.contains("BGP") {
        return Ok(State::Start);
    }

    // Parse neighbor header line using regex match
    let caps = RE_NEIGHBOR_HEADER.captures(line);
    let next_state = if let Some(caps) = caps {
        neighbor.id = caps["protocol"].to_string();
        // State
        neighbor.state = caps["state"].to_string().to_lowercase();
        if neighbor.state == "down" {
            neighbor.last_error = caps["info"].to_string();
        }
        // Uptime
        neighbor.uptime = datetime::parse_duration_sec(&caps["uptime"])?;
        neighbor.since = datetime::parse(&caps["uptime"])?;

        State::Meta
    } else {
        State::Start
    };

    Ok(next_state)
}

/// Parse neighbor meta: Description,
fn parse_neighbor_meta(neighbor: &mut Neighbor, line: &str) -> Result<State> {
    // Parse description
    let caps = RE_KEY_VALUE.captures(line);
    if let Some(caps) = caps {
        neighbor.description = caps["value"].to_string();
    }

    Ok(State::BgpState)
}

/// ParseBGP State
fn parse_bgp_state(neighbor: &mut Neighbor, line: &str) -> Result<State> {
    // This is a collection of key value pairs.
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if key == "neighbor address" {
            neighbor.address = val
        } else if key == "neighbor as" {
            neighbor.asn = val.parse::<u32>()?;
        } else if key == "route change stats" {
            // We found the next segment
            return Ok(State::RouteChangeStats);
        }
    }

    Ok(State::BgpState)
}

/// Change Stats
struct ChangeStats {
    received: u32,
    accepted: u32,
    rejected: u32,
    filtered: u32,
}

impl ChangeStats {
    fn parse(row: &str) -> Result<ChangeStats> {
        let parts: Vec<&str> = row.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(anyhow!("Invalid change stats row: {}", row));
        }

        Ok(ChangeStats {
            received: parts[0].parse().unwrap_or(0),
            rejected: parts[1].parse().unwrap_or(0),
            filtered: parts[2].parse().unwrap_or(0),
            accepted: parts[4].parse().unwrap_or(0),
        })
    }
}

/// Parse route change stats
fn parse_route_change_stats(neighbor: &mut Neighbor, line: &str) -> Result<State> {
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if key == "import updates" {
            let stats = ChangeStats::parse(&val)?;
            neighbor.routes_received = stats.received;
            neighbor.routes_filtered = stats.filtered;
            neighbor.routes_accepted = stats.accepted;
        } else if key == "export updates" {
            let stats = ChangeStats::parse(&val)?;
            neighbor.routes_exported = stats.received - stats.rejected - stats.filtered;
        }
    }

    Ok(State::RouteChangeStats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    #[test]
    fn test_parse_neighbor_header() {
        let line = "1002-R194_42    BGP        ---        up     09:39:25.123  Established";
        let mut neighbor = Neighbor::default();
        parse_neighbor_header(&mut neighbor, &line).unwrap();

        assert_eq!(neighbor.id, "R194_42");
        assert_eq!(neighbor.state, "up");
        assert!(neighbor.uptime > 0.0);
    }

    #[test]
    fn test_parse_neighbor_header_down() {
        let line = "1002-R_bhac01   BGP        ---        down   2023-04-19 09:08:10  Error: No listening socket";
        let mut neighbor = Neighbor::default();
        parse_neighbor_header(&mut neighbor, &line).unwrap();

        assert_eq!(neighbor.id, "R_bhac01");
        assert_eq!(neighbor.state, "down");
        assert_eq!(neighbor.last_error, "Error: No listening socket");
    }

    #[test]
    fn test_parse_neighbor_header_idle() {
        let line = "1002-R192_158   BGP        ---        start  2023-04-20 12:01:52  Idle          BGP Error: Bad peer AS";
        let mut neighbor = Neighbor::default();
        parse_neighbor_header(&mut neighbor, &line).unwrap();
    }

    #[test]
    fn test_parse_neighbor_meta() {
        let line = "1006-  Description:    AnniNET Software Development";
        let mut neighbor = Neighbor::default();
        parse_neighbor_meta(&mut neighbor, &line).unwrap();
        assert_eq!(neighbor.description, "AnniNET Software Development");
    }

    #[test]
    fn test_parse_neighbor_bgpstate() {
        let mut neighbor = Neighbor::default();
        let line = "   BGP state:          Established ";
        let next = parse_bgp_state(&mut neighbor, &line).unwrap();
        assert_eq!(next, State::BgpState);

        let line = "   Neighbor address: 172.31.194.42";
        parse_bgp_state(&mut neighbor, &line).unwrap();
        let line = "     Neighbor AS:      42";
        parse_bgp_state(&mut neighbor, &line).unwrap();

        assert_eq!(neighbor.address, "172.31.194.42");
        assert_eq!(neighbor.asn, 42);

        let line =
            "     Route change stats:     received   rejected   filtered    ignored   accepted";
        let next = parse_bgp_state(&mut neighbor, &line).unwrap();
        assert_eq!(next, State::RouteChangeStats);
    }

    #[test]
    fn test_neighbor_parse() {
        let block: Block = vec![
            "1002-R194_42    BGP        ---        up     2023-04-19 09:39:25  Established".into(),
            "1006-  Description:    Packet Clearing House".into(),
            "   BGP state:          Established".into(),
            "    Neighbor address: 172.31.194.42".into(),
            "    Neighbor AS:      42".into(),
        ];
        let neighbor = Neighbor::parse(block).unwrap();
        assert_eq!(neighbor.id, "R194_42");
    }

    #[test]
    fn test_parse_neighbors() {
        let input = File::open("tests/birdc/show-protocols-all").unwrap();
        let reader = BufReader::new(input);
        let neighbors = Neighbor::read(reader).unwrap();

        let neighbor = &neighbors[0];
        assert_eq!(neighbor.id, "R194_42");
        assert_eq!(neighbor.address, "172.31.194.42");
    }
}
