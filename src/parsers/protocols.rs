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

pub struct ProtocolReader<R: BufRead> {
    iter: BlockIterator<R>,
}

impl<R: BufRead> ProtocolReader<R> {
    pub fn new(reader: R) -> Self {
        let iter = BlockIterator::new(reader, &RE_PROTOCOL_START);
        Self { iter }
    }
}

impl<R: BufRead> Iterator for ProtocolReader<R> {
    type Item = Protocol;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.iter.next()?;
        match Protocol::parse(block) {
            Ok(protocol) => Some(protocol),
            Err(e) => {
                tracing::error!(
                    error = e.to_string(),
                    "parsing protocol failed"
                );
                Some(Protocol::default())
            }
        }
    }
}

/// Implement block parser for protocol
impl Parse<Block> for Protocol {
    /// Parse a block of lines into a protocol
    fn parse(block: Block) -> Result<Self> {
        let mut protocol = Protocol::default();

        // Parse lines in block
        let mut state = State::Start;
        for line in block.iter() {
            match parse_line(&mut protocol, state.clone(), &line) {
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
) -> Result<State> {
    let state = match state {
        State::Start => parse_protocol_header(&mut protocol, line)?,
        State::Meta => parse_protocol_meta(&mut protocol, line)?,
        State::BgpState => parse_bgp_state(&mut protocol, line)?,
        State::Channel(ch, sec) => {
            parse_channel(&mut protocol, ch, sec, line)?
        }
        State::Done => State::Done,
    };
    Ok(state)
}

/// Parse Protocol Header (name, state, uptime) and return next state
fn parse_protocol_header(
    protocol: &mut Protocol,
    line: &str,
) -> Result<State> {
    // Parse protocol header line using regex match
    let caps = RE_PROTOCOL_HEADER.captures(line);
    let next_state = if let Some(caps) = caps {
        protocol.id = caps["protocol"].to_string();

        // State
        protocol.state = caps["state"].to_string().to_lowercase();
        if protocol.state == "down" {
            protocol.last_error = caps["info"].to_string();
        }

        // Uptime
        protocol.since = caps["uptime"].trim().into();
        protocol.state_changed = caps["uptime"].trim().into();

        State::Meta
    } else {
        State::Start
    };

    Ok(next_state)
}

/// Parse protocol meta: Description,
fn parse_protocol_meta(protocol: &mut Protocol, line: &str) -> Result<State> {
    {
        if let Some(caps) = RE_PROTOCOL_CHANNEL.captures(line) {
            let channel = caps["channel"].to_string();
            return Ok(State::Channel(channel, ChannelSection::Meta));
        }
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
    {
        if let Some(caps) = RE_PROTOCOL_CHANNEL.captures(line) {
            let channel = caps["channel"].to_string();
            return Ok(State::Channel(channel, ChannelSection::Meta));
        }
    }

    // This is a collection of key value pairs.
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if key == "protocol address" {
            protocol.address = val
        } else if key == "protocol as" {
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
        Ok(State::Done)
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
