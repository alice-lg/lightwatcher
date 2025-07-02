use regex::Regex;
use std::io::BufRead;

use anyhow::Result;
use lazy_static::lazy_static;

use crate::{
    bird::{Channel, Protocol, RouteChangeStats, RoutesCount},
    parsers::parser::{Block, BlockIterator, Parse},
};

lazy_static! {
    /// Regex for start protocol
    static ref RE_PROTOCOL_START: Regex = Regex::new(r"1002-").unwrap();

    /// Regex: Protocol header (protocol, state, uptime, ...)
    static ref RE_PROTOCOL_HEADER: Regex = Regex::new(r"(?x)
        1002-(?P<protocol>\w+)   # protocol id
        \s+(?P<type>\w+)         # type
        \s+.*?\s+                # ignore this part
        (?P<state>\w+)           # state (up / down)
        \s+
        (?P<uptime>[\d\-:\s]+)    # since
        (\.\d+)?\s+?              # trailing time
        (?P<info>.*)$             # additional info
    ").unwrap();

    /// Regex: Channel
    static ref RE_PROTOCOL_CHANNEL: Regex = Regex::new(r".* [Cc]hannel (?P<channel>.*)").unwrap();

    /// Regex for a Key: Value pair
    static ref RE_KEY_VALUE: Regex = Regex::new(r"(?x)
        .*?\s+
        (?P<key>[\s\w]+):
        \s+
        (?P<value>.+)
    ").unwrap();
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
}

pub struct ProtocolReader<R: BufRead> {
    iter: BlockIterator<R>,
    filter_bgp: bool,
}

impl<R: BufRead> ProtocolReader<R> {
    pub fn new(reader: R) -> Self {
        let iter = BlockIterator::new(reader, &RE_PROTOCOL_START);
        Self {
            iter,
            filter_bgp: false,
        }
    }

    /// Setting the filter bgp flag will skip all non 'BGP'
    /// protocols.
    pub fn with_filter_bgp(self) -> Self {
        Self {
            filter_bgp: true,
            ..self
        }
    }
}

impl<R: BufRead> Iterator for ProtocolReader<R> {
    type Item = Protocol;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.iter.next()?;
        match Protocol::parse(block, self.filter_bgp) {
            Ok(protocol) => Some(protocol),
            Err(e) => {
                tracing::error!(
                    error = e.to_string(),
                    "parsing protocol failed"
                );
                // Some(Protocol::default())
                self.next()
            }
        }
    }
}

/// Implement block parser for protocol
impl Protocol {
    /// Parse a block of lines into a protocol
    fn parse(block: Block, filter_bgp: bool) -> Result<Self> {
        let mut protocol = Protocol::default();

        // Parse lines in block
        let mut state = State::Start;
        for line in block.iter() {
            match parse_line(&mut protocol, state.clone(), &line, filter_bgp) {
                Ok(next_state) => state = next_state,
                Err(e) => {
                    tracing::error!(
                        line = line,
                        error = e.to_string(),
                        state = format!("{:?}", state),
                        protocol = format!("{:?}", protocol),
                        "failed parsing line"
                    );
                    return Err(e);
                }
            }
        }

        // Finalize protocol for compatibility: Update number
        // of routes according to stats in the channel.
        finalize_counters(&mut protocol);
        finalize_attributes(&mut protocol);

        Ok(protocol)
    }
}

/// Parse input depending on the current state
fn parse_line(
    mut protocol: &mut Protocol,
    state: State,
    line: &str,
    filter_bgp: bool,
) -> Result<State> {
    let state = match state {
        State::Start => {
            parse_protocol_header(&mut protocol, line, filter_bgp)?
        }
        State::Meta => parse_protocol_meta(&mut protocol, line)?,
        State::BgpState => parse_bgp_state(&mut protocol, line)?,
        State::Channel(ch, sec) => {
            parse_channel(&mut protocol, ch, sec, line)?
        }
    };
    Ok(state)
}

/// Parse Protocol Header (name, state, uptime) and return next state
fn parse_protocol_header(
    protocol: &mut Protocol,
    line: &str,
    filter_bgp: bool,
) -> Result<State> {
    if filter_bgp && !line.contains("BGP") {
        return Ok(State::Start);
    }

    // Parse protocol header line using regex match
    let caps = RE_PROTOCOL_HEADER.captures(line);
    let next_state = if let Some(caps) = caps {
        protocol.id = caps["protocol"].to_string();
        protocol.bird_protocol = caps["type"].to_string();

        // State
        protocol.state = caps["state"].to_string().to_lowercase();
        if protocol.state == "down" {
            protocol.last_error = caps["info"].to_string();
        }

        // Uptime
        let uptime = caps["uptime"].trim();
        protocol.since = uptime.into();
        protocol.state_changed = uptime.into();

        State::Meta
    } else {
        State::Start
    };

    Ok(next_state)
}

/// Check if the line marks the beginning of a new
/// channel section.
fn parse_channel_header(l: &str) -> Option<String> {
    match RE_PROTOCOL_CHANNEL.captures(l) {
        Some(caps) => Some(caps["channel"].to_string()),
        None => None,
    }
}

/// Parse protocol meta: Description,
fn parse_protocol_meta(protocol: &mut Protocol, line: &str) -> Result<State> {
    if let Some(channel) = parse_channel_header(line) {
        return Ok(State::Channel(channel, ChannelSection::Meta));
    }

    // Parse description
    let caps = RE_KEY_VALUE.captures(line);
    if let Some(caps) = caps {
        let key = &caps["key"].to_lowercase();
        if key == "description" {
            protocol.description = caps["value"].to_string();
        }
    }

    Ok(State::BgpState)
}

/// ParseBGP State
fn parse_bgp_state(protocol: &mut Protocol, line: &str) -> Result<State> {
    // Check if we reached a channel section, so we can continue with
    // the next parser state:
    if let Some(channel) = parse_channel_header(line) {
        return Ok(State::Channel(channel, ChannelSection::Meta));
    }

    // This is a collection of key value pairs.
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if key == "neighbor address" {
            protocol.address = val
        } else if key == "neighbor as" {
            protocol.asn = val.parse::<u32>()?;
        }
    }

    Ok(State::BgpState)
}

/// Parse per channel information
fn parse_channel(
    protocol: &mut Protocol,
    channel: String,
    section: ChannelSection,
    line: &str,
) -> Result<State> {
    if let Some(next) = parse_channel_header(line) {
        return Ok(State::Channel(next, ChannelSection::Meta));
    }
    match section {
        ChannelSection::Meta => parse_channel_meta(protocol, channel, line),
        ChannelSection::RouteChangeStats(fields) => {
            parse_channel_route_change_stats(protocol, channel, fields, line)
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
fn parse_change_stats_values(s: &str) -> Vec<Option<u32>> {
    s.split_whitespace()
        .map(|v| if let Ok(v) = v.parse() { Some(v) } else { None })
        .collect()
}

/// Parse channel metadata like
/// state, import, export, table, etc...
fn parse_channel_meta(
    protocol: &mut Protocol,
    channel: String,
    line: &str,
) -> Result<State> {
    let chan = protocol
        .channels
        .entry(channel.clone())
        .or_insert(Channel::default());

    if let Some(caps) = RE_KEY_VALUE.captures(&line) {
        let key = caps["key"].to_lowercase().to_string();
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
        } else if key == "peer table" {
            chan.peer_table = val;
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
    protocol: &mut Protocol,
    channel: String,
    fields: Vec<String>,
    line: &str,
) -> Result<State> {
    let chan = protocol
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
        Ok(State::Meta)
    }
}

/// Finalize counts: As we accept routes through multiple channels
/// e.g. IPv4 and IPv6 the global counter object `routes` has to
/// be calculated after parsing the protocol.
fn finalize_counters(protocol: &mut Protocol) {
    // We assume that the total number of routes received, filtered, preferred, ...
    // is the sum over all channels. TODO: validate.
    let mut total = RoutesCount::default();
    for (_, chan) in protocol.channels.iter() {
        for (key, count) in &chan.routes_count {
            total
                .entry(key.into())
                .and_modify(|c| *c += count)
                .or_insert(count.clone());
        }
    }

    protocol.routes = total;
}

/// Some attributes need to be present on the
/// root level of the parsed protocol.
fn finalize_attributes(protocol: &mut Protocol) {
    // Get compatibility attributes from first channel
    for (_, attrs) in protocol.channels.iter() {
        protocol.table = attrs.table.clone();
        protocol.peer_table = attrs.peer_table.clone();
        break;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::BufReader};

    #[test]
    fn test_parse_protocol_header() {
        let line = "1002-R194_42    BGP        ---        up     09:39:25.123  Established";
        let mut protocol = Protocol::default();
        parse_protocol_header(&mut protocol, &line, false).unwrap();

        assert_eq!(protocol.id, "R194_42");
        assert_eq!(protocol.state, "up");
    }

    #[test]
    fn test_parse_protocol_header_date() {
        let line = "1002-R194_42    BGP        ---        up     2025-05-27  Established";
        let mut protocol = Protocol::default();
        parse_protocol_header(&mut protocol, &line, false).unwrap();

        assert_eq!(protocol.id, "R194_42");
        assert_eq!(protocol.state, "up");
    }

    #[test]
    fn test_parse_protocol_header_down() {
        let line = "1002-R_bhac01   BGP        ---        down   2023-04-19 09:08:10  Error: No listening socket";
        let mut protocol = Protocol::default();
        parse_protocol_header(&mut protocol, &line, false).unwrap();

        assert_eq!(protocol.id, "R_bhac01");
        assert_eq!(protocol.state, "down");
        assert_eq!(protocol.last_error, "Error: No listening socket");
    }

    #[test]
    fn test_parse_protocol_header_idle() {
        let line = "1002-R192_158   BGP        ---        start  2023-04-20 12:01:52  Idle          BGP Error: Bad peer AS";
        let mut protocol = Protocol::default();
        parse_protocol_header(&mut protocol, &line, false).unwrap();
    }

    #[test]
    fn test_parse_protocol_meta() {
        let line = "1006-  Description:    AnniNET Software Development";
        let mut protocol = Protocol::default();
        parse_protocol_meta(&mut protocol, &line).unwrap();
        assert_eq!(protocol.description, "AnniNET Software Development");
    }

    #[test]
    fn test_parse_protocol_bgpstate() {
        let mut protocol = Protocol::default();
        let line = "   BGP state:          Established ";
        let next = parse_bgp_state(&mut protocol, &line).unwrap();
        assert_eq!(next, State::BgpState);

        let line = "   neighbor address: 172.31.194.42";
        parse_bgp_state(&mut protocol, &line).unwrap();
        let line = "     neighbor AS:      42";
        parse_bgp_state(&mut protocol, &line).unwrap();

        assert_eq!(protocol.address, "172.31.194.42");
        assert_eq!(protocol.asn, 42);
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
        assert_eq!(values[0], Some(471));
        assert_eq!(values[1], Some(47));
        assert_eq!(values[4], None);
        assert_eq!(values[6], Some(412));
    }

    #[test]
    fn test_protocol_parse() {
        let block: Block = vec![
            "1002-R194_42    BGP        ---        up     2023-04-19 09:39:25  Established".into(),
            "1006-  Description:    Packet Clearing House".into(),
            "   BGP state:          Established".into(),
            "    Neighbor address: 172.31.194.42".into(),
            "    Neighbor AS:      42".into(),
        ];
        let protocol = Protocol::parse(block, false).unwrap();
        assert_eq!(protocol.id, "R194_42");
        assert_eq!(protocol.address, "172.31.194.42");
        assert_eq!(protocol.asn, 42);
    }

    #[test]
    fn test_protocol_reader() {
        let input = File::open("tests/birdc/show-protocols-all").unwrap();
        let buf = BufReader::new(input);
        let reader = ProtocolReader::new(buf).with_filter_bgp();
        let protocols: Vec<Protocol> =
            reader.filter(|n| !n.id.is_empty()).collect();

        // Let's check the first BGP protocol
        let protocol = &protocols[0];
        println!("PRO: {:?}", protocol);
        assert_eq!(protocol.id, "R194_42");
        assert_eq!(protocol.address, "111.111.194.42");

        println!("ROUTES: {:?}", protocol.routes);
        assert_eq!(protocol.routes["imported"], 110);
    }
}
