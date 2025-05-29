use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::io::BufRead;

use crate::{
    parsers::{
        datetime,
        parser::{Block, BlockIterator, Parse},
    },
    route_server::{Channel, Neighbor, RouteChangeStats, RoutesCount},
};

lazy_static! {
    /// Regex for start neighbor
    static ref RE_NEIGHBOR_START: Regex = Regex::new(r"1002-").unwrap();

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

/// Channel sections
#[derive(Debug, PartialEq, Clone)]
enum ChannelSection {
    Meta,
    RouteChangeStats(Vec<String>),
}

/// Parser sections
#[derive(Debug, PartialEq, Clone)]
enum State {
    Start,
    Meta,
    BgpState,
    Channel(String, ChannelSection),
    Done,
}

pub struct NeighborReader<R: BufRead> {
    iter: BlockIterator<R>,
}

impl<R: BufRead> NeighborReader<R> {
    pub fn new(reader: R) -> Self {
        let iter = BlockIterator::new(reader, &RE_NEIGHBOR_START);
        Self { iter }
    }
}

impl<R: BufRead> Iterator for NeighborReader<R> {
    type Item = Neighbor;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.iter.next()?;
        match Neighbor::parse(block) {
            Ok(neighbor) => Some(neighbor),
            Err(e) => {
                tracing::error!(
                    error = e.to_string(),
                    "parsing neighbor failed"
                );
                Some(Neighbor::default())
            }
        }
    }
}

/// Implement block parser for neighbor
impl Parse<Block> for Neighbor {
    /// Parse a block of lines into a neighbor
    fn parse(block: Block) -> Result<Self> {
        let mut neighbor = Neighbor::default();

        // Parse lines in block
        let mut state = State::Start;
        for line in block.iter() {
            match parse_line(&mut neighbor, state.clone(), &line) {
                Ok(next_state) => state = next_state,
                Err(e) => {
                    tracing::error!(
                        line = line,
                        error = e.to_string(),
                        state = format!("{:?}", state),
                        neighbor = format!("{:?}", neighbor),
                        "failed parsing line"
                    );
                    return Err(e);
                }
            }
        }

        // Finalize neighbor for compatibility: Update number
        // of routes according to stats in the channel.

        Ok(neighbor)
    }
}

/// Parse input depending on the current state
fn parse_line(
    mut neighbor: &mut Neighbor,
    state: State,
    line: &str,
) -> Result<State> {
    let state = match state {
        State::Start => parse_neighbor_header(&mut neighbor, line)?,
        State::Meta => parse_neighbor_meta(&mut neighbor, line)?,
        State::BgpState => parse_bgp_state(&mut neighbor, line)?,
        State::Channel(ch, sec) => {
            parse_channel(&mut neighbor, ch, sec, line)?
        }
        State::Done => State::Done,
    };
    Ok(state)
}

/// Parse Neighbor Header (name, state, uptime) and return next state
fn parse_neighbor_header(
    neighbor: &mut Neighbor,
    line: &str,
) -> Result<State> {
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
        neighbor.state_changed = caps["uptime"].trim().into();

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
        let key = &caps["key"].to_lowercase();
        if key == "description" {
            neighbor.description = caps["value"].to_string();
        }
    }

    Ok(State::BgpState)
}

/// ParseBGP State
fn parse_bgp_state(neighbor: &mut Neighbor, line: &str) -> Result<State> {
    // Check if we reached a channel section, so we can continue with
    // the next parser state:
    {
        let line = line.trim().to_lowercase();
        if let Some(channel) = line.strip_prefix("channel ") {
            return Ok(State::Channel(channel.into(), ChannelSection::Meta));
        }
    }

    // This is a collection of key value pairs.
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if key == "neighbor address" {
            neighbor.address = val
        } else if key == "neighbor as" {
            neighbor.asn = val.parse::<u32>()?;
        }
    }

    Ok(State::BgpState)
}

impl Parse<&str> for RoutesCount {
    fn parse(row: &str) -> Result<RoutesCount> {
        let parts = row.split(",");
        let count: RoutesCount = parts
            .map(|s| {
                let s: Vec<&str> = s.trim().split_whitespace().collect();
                if s.len() != 2 {
                    tracing::error!("could not parse routes count");
                }
                if let Ok(val) = s[0].parse() {
                    (s[1].into(), val)
                } else {
                    (s[1].into(), 0)
                }
            })
            .collect();

        Ok(count)
    }
}

/// Parse per channel information
fn parse_channel(
    neighbor: &mut Neighbor,
    channel: String,
    section: ChannelSection,
    line: &str,
) -> Result<State> {
    match section {
        ChannelSection::Meta => parse_channel_meta(neighbor, channel, line),
        ChannelSection::RouteChangeStats(fields) => {
            parse_channel_route_change_stats(neighbor, channel, fields, line)
        }
    }
}

/// Get the fields from the change stats header.
fn parse_change_stats_fields(s: &str) -> Vec<String> {
    s.split("  ")
        .filter_map(|f| {
            if f != "" {
                let f = f.trim().to_lowercase().replace(" ", "_").into();
                return Some(f);
            }
            None
        })
        .collect()
}

/// Parse field values
fn parse_change_stats_values(s: &str) -> Vec<u32> {
    s.split_whitespace()
        .filter_map(|v| {
            if let Ok(v) = v.parse() {
                Some(v)
            } else {
                Some(0)
            }
        })
        .collect()
}

/// Parse channel metadata like
/// state, import, export, table, etc...
fn parse_channel_meta(
    neighbor: &mut Neighbor,
    channel: String,
    line: &str,
) -> Result<State> {
    let chan = neighbor
        .channels
        .entry(channel.clone())
        .or_insert(Channel::default());

    let line = line.to_lowercase();
    if let Some(caps) = RE_KEY_VALUE.captures(&line) {
        let key = caps["key"].to_string();
        let val = caps["value"].to_string();

        // Match keys
        if key == "state" {
            chan.state = val;
        } else if key == "import state" {
            chan.import_state = val;
        } else if key == "export state" {
            chan.export_state = val;
        } else if key == "table" {
            chan.table = val;
        } else if key == "preference" {
            chan.preference = val.parse()?;
        } else if key == "input filter" {
            chan.input_filter = val;
        } else if key == "output filter" {
            chan.output_filter = val;
        } else if key == "routes" {
            chan.routes_count = RoutesCount::parse(&val)?;
        } else if key == "bgp next hop" {
            chan.bgp_next_hop = val;
        } else if key == "route change stats" {
            let fields = parse_change_stats_fields(&val);
            return Ok(State::Channel(
                channel,
                ChannelSection::RouteChangeStats(fields),
            ));
        }
    }

    Ok(State::Channel(channel, ChannelSection::Meta))
}

/// Parse channel route change stats
fn parse_channel_route_change_stats(
    neighbor: &mut Neighbor,
    channel: String,
    fields: Vec<String>,
    line: &str,
) -> Result<State> {
    let chan = neighbor
        .channels
        .entry(channel.clone())
        .or_insert(Channel::default());

    let line = line.to_lowercase();
    if let Some(caps) = RE_KEY_VALUE.captures(&line) {
        let key = &caps["key"];
        let val = &caps["value"];

        // This is not great and should be handled by the meta parsing.
        if key == "bgp next hop" {
            chan.bgp_next_hop = val.into();
            return Ok(State::Channel(
                channel,
                ChannelSection::RouteChangeStats(fields),
            ));
        }

        let values = parse_change_stats_values(val);
        let stats: RouteChangeStats =
            fields.clone().into_iter().zip(values.into_iter()).collect();

        if key == "import updates" {
            chan.route_change_stats.import_updates = stats;
        } else if key == "import withdraws" {
            chan.route_change_stats.import_withdraws = stats;
        } else if key == "export updates" {
            chan.route_change_stats.export_updates = stats;
        } else if key == "export withdraws" {
            chan.route_change_stats.export_withdraws = stats;
        }

        Ok(State::Channel(
            channel,
            ChannelSection::RouteChangeStats(fields),
        ))
    } else {
        Ok(State::Done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::BufReader};

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
    fn test_parse_neighbor_header_date() {
        let line = "1002-R194_42    BGP        ---        up     2025-05-27  Established";
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
    }

    #[test]
    fn test_parse_change_stats_fields() {
        let s = "     received   rejected   filtered    ignored   RX limit      limit   accepted";
        let fields = parse_change_stats_fields(s);

        assert_eq!(fields[0], "received");
        assert_eq!(fields[4], "rx_limit");
        assert_eq!(fields[5], "limit");
    }

    #[test]
    fn test_parse_change_stats_values() {
        let s = "            471         47         12          0        ---          0        412";
        let values = parse_change_stats_values(s);
        assert_eq!(values[0], 471);
        assert_eq!(values[1], 47);
        assert_eq!(values[4], 0);
        assert_eq!(values[6], 412);
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
    fn test_neighbor_reader() {
        let input = File::open("tests/birdc/show-protocols-all").unwrap();
        let buf = BufReader::new(input);
        let reader = NeighborReader::new(buf);
        let neighbors: Vec<Neighbor> =
            reader.filter(|n| !n.id.is_empty()).collect();

        let neighbor = &neighbors[0];
        assert_eq!(neighbor.id, "R194_42");
        assert_eq!(neighbor.address, "111.111.194.42");
    }
}
