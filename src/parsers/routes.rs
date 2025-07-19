use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    bird::{Community, ExtCommunity, LargeCommunity, Route},
    parsers::parser::{Block, BlockGroup, Parse},
};

lazy_static! {
    /// Match a route header
    static ref RE_ROUTE_HEADER: Regex = Regex::new(
        r"(?x)
          .*?
          (?P<prefix>[0-9a-f:\./]+)?\s+   # Network
          (?P<type>\w+)\s+ 
          \[
            (?P<from_protocol>.*?)\s+(?P<age>[\d\-:\.\s]+)
            (\s+from\s+(?P<learnt_from>.+))?
          \]\s+
          ((?P<primary>\*)\s+)?
          \((?P<metric>\d+)\)\s+
          .*$
    "
    )
    .unwrap();

    static ref RE_GATEWAY_INTERFACE: Regex = Regex::new(
        r"(?x)
          .*?via\s+
          (?P<gateway>[0-9a-f:\.]+)?\s+   # Gateway
          on\s+
          (?P<interface>.+)
        ").unwrap();

    /// Regex for a Key: Value pair
    static ref RE_KEY_VALUE: Regex = Regex::new(r"(?x)
        .*?\s+
        (?P<key>[\s\w\.]+):
        \s+
        (?P<value>.+)
    ").unwrap();

    /// BGP Community Regex
    static ref RE_BGP_COMMUNITY: Regex = Regex::new(r"\((.+?), (.+?), (.+?)\)\s?").unwrap();

    pub static ref RE_ROUTES_START: Regex = Regex::new(r"1007-\S").unwrap();
    static ref RE_ROUTE_START: Regex = Regex::new(r"1007-").unwrap();
}


#[derive(Debug, PartialEq)]
enum BGPCommunities {
    Regular,
    Large,
    Extended,
}

/// Current BGP information we are parsing.
/// These might be attributes or communities.
///
/// We need to keep track of the current state as the
/// communities may span multiple lines.
#[derive(Debug, PartialEq)]
enum BGPState {
    Attributes,
    Communities(BGPCommunities),
}

/// Route Parser State
#[derive(Debug, PartialEq)]
enum State {
    Start,
    Meta,
    BGP(BGPState),
}


/// A routes group that shares the same prefix. However while parsing
/// only the first route has a prefix.
pub type PrefixGroup = Vec<Route>;

impl Parse<Block> for PrefixGroup {
    fn parse(block: Block) -> Result<Self> {
        let mut routes: PrefixGroup = Vec::new();
        let iter = BlockGroup::new(block, &RE_ROUTE_START);
        let mut prefix: String = String::new(); // Current prefix

        for block in iter {
            if block[0].starts_with("0001") {
                continue;
            }
            let mut route = Route::parse(block)?;
            if route.network.is_empty() {
                route.network = prefix.clone();
            } else {
                prefix = route.network.clone();
            }

            if route.neighbor_id.is_none() {
                continue; // ??
            }
            routes.push(route);
        }

        Ok(routes)
    }
}

/// Implement Parse for route
impl Parse<Block> for Route {
    fn parse(block: Block) -> Result<Self> {
        let mut route = Route::default();
        let mut state = State::Start;
        for line in block.iter() {
            match parse_line(&mut route, state, line) {
                Ok(next_state) => state = next_state,
                Err(e) => {
                    tracing::error!(
                        line = line,
                        error = e.to_string(),
                        "failed parsing line"
                    );
                    return Err(e);
                }
            }
        }
        Ok(route)
    }
}

/// Parse a line in a block
fn parse_line(route: &mut Route, state: State, line: &str) -> Result<State> {
    match state {
        State::Start => parse_route_header(route, line),
        State::Meta => parse_route_meta(route, line),
        State::BGP(bgp_state) => parse_route_bgp(route, bgp_state, line),
    }
}

/// Parse route header
fn parse_route_header(route: &mut Route, line: &str) -> Result<State> {
    let caps = RE_ROUTE_HEADER.captures(line);
    if let Some(caps) = caps {
        if let Some(prefix) = caps.name("prefix") {
            route.network = prefix.as_str().to_string();
        }
        if let Some(age) = caps.name("age") {
            // route.age = datetime::parse_duration_sec(age.as_str())?;
            route.age = age.as_str().to_string();
        }
        if caps.name("primary").is_some() {
            route.primary = true;
        }
        if let Some(metric) = caps.name("metric") {
            route.metric = metric.as_str().parse::<u32>()?;
        }
        if let Some(from) = caps.name("learnt_from") {
            route.learnt_from = Some(from.as_str().to_string());
        }
        if let Some(proto) = caps.name("from_protocol") {
            route.neighbor_id = Some(proto.as_str().to_string());
        }

        return Ok(State::Meta);
    }

    Ok(State::Start)
}

/// Parse route type (list of strings)
fn parse_route_type(s: &str) -> Result<Vec<String>> {
    let route_types = s.split(" ").map(|s| s.to_string()).collect();
    Ok(route_types)
}

/// Parse route meta
fn parse_route_meta(route: &mut Route, line: &str) -> Result<State> {
    let caps = RE_GATEWAY_INTERFACE.captures(line);
    if let Some(caps) = caps {
        if let Some(gateway) = caps.name("gateway") {
            route.gateway = gateway.as_str().to_string();
        }
        if let Some(interface) = caps.name("interface") {
            route.interface = interface.as_str().to_string();
        }
        return Ok(State::Meta);
    }

    let caps = RE_KEY_VALUE.captures(line);
    if let Some(caps) = caps {
        if let Some(key) = caps.name("key") {
            if let Some(value) = caps.name("value") {
                if key.as_str() == "Type" {
                    route.route_type = parse_route_type(value.as_str())?;
                }
            }
        }
    }

    Ok(State::BGP(BGPState::Attributes))
}

/// Parse AS path
fn parse_as_path(s: &str) -> Result<Vec<String>> {
    /*
    let mut as_path: Vec<String> = vec![];
    for asn in s.split(" ") {
        as_path.push(asn);
    }
    */
    // To keep this backwards compatible this needs to
    // be stringly typed. Sigh.
    let as_path: Vec<String> = s.split(" ").map(|p| p.to_string()).collect();
    Ok(as_path)
}

/// Parse a list separated by spaces
fn parse_list<T>(s: &str, parse: fn(&str) -> Result<T>) -> Result<Vec<T>> {
    let s = s.trim();
    let mut list: Vec<T> = vec![];
    for item in s.split(" ") {
        list.push(parse(item)?);
    }
    Ok(list)
}

/// Parse BGP community
fn parse_community(s: &str) -> Result<Community> {
    let s = s[1..s.len() - 1].to_string(); // Strip braces
    let tokens: Vec<&str> = s.split(",").collect();
    if tokens.len() != 2 {
        return Err(anyhow!("Invalid community: {}", s));
    }
    Ok(Community(tokens[0].parse()?, tokens[1].parse()?))
}

/// Parse a list of communities
fn parse_communities(s: &str) -> Result<Vec<Community>> {
    parse_list(s, parse_community)
}

/// Parse a list of ext communities
fn parse_ext_communities(s: &str) -> Result<Vec<ExtCommunity>> {
    let communities: Vec<ExtCommunity> = RE_BGP_COMMUNITY
        .captures_iter(s)
        .map(|c| {
            ExtCommunity(c[1].to_string(), c[2].to_string(), c[3].to_string())
        })
        .collect();
    Ok(communities)
}

/// Parse a list of large communities
fn parse_large_communities(s: &str) -> Result<Vec<LargeCommunity>> {
    let communities: Vec<LargeCommunity> = RE_BGP_COMMUNITY
        .captures_iter(s)
        .map(|c| {
            LargeCommunity(
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
                c[3].parse().unwrap(),
            )
        })
        .collect();
    Ok(communities)
}


fn parse_route_bgp_communities(
    route: &mut Route,
    ctype: BGPCommunities,
    line: &str,
) -> Result<State> {
    let mut line = line.trim_start().to_lowercase();

    // Strip everything before the colon
    if let Some(index) = line.find(':') {
        line = (&line[index + 1..]).to_string();
    }

    // Append to existing list of communities
    match ctype {
        BGPCommunities::Regular => {
            route
                .bgp
                .communities
                .append(&mut parse_communities(&line)?);
        }
        BGPCommunities::Large => {
            route
                .bgp
                .large_communities
                .append(&mut parse_large_communities(&line)?);
        }
        BGPCommunities::Extended => {
            route
                .bgp
                .ext_communities
                .append(&mut parse_ext_communities(&line)?);
        }
    }

    Ok(State::BGP(BGPState::Communities(ctype)))
}


/// Parse route BGP
fn parse_route_bgp(route: &mut Route, state: BGPState, line: &str) -> Result<State> {

    // Parse key value info
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if !key.starts_with("bgp") {
            return Ok(State::BGP(BGPState::Attributes));
        }
        let key = &key[4..];

        if key == "origin" {
            route.bgp.origin = val;
        } else if key == "as_path" || key == "path" {
            route.bgp.as_path = parse_as_path(&val)?;
        } else if key == "next_hop" {
            route.bgp.next_hop = val;
        } else if key == "otc" {
            route.bgp.otc = Some(val);
        } else if key == "med" {
            route.bgp.med = val;
        } else if key == "local_pref" {
            route.bgp.local_pref = val;
        } else if key == "community" {
            return parse_route_bgp_communities(route, BGPCommunities::Regular, line); 
        } else if key == "large_community" {
            return parse_route_bgp_communities(route, BGPCommunities::Large, line);
        } else if key == "ext_community" {
            return parse_route_bgp_communities(route, BGPCommunities::Extended, line);
        }

        Ok(State::BGP(BGPState::Attributes))
    } else {
        // We might be in a communities continuation.
        match state {
            BGPState::Communities(ctype) => parse_route_bgp_communities(route, ctype, line),
            BGPState::Attributes => Ok(State::BGP(BGPState::Attributes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_route_header() {
        let line =
            "1007-203.17.254.0/24      unicast [R192_172 2023-04-19 09:28:42] * (100) [AS7545i]";
        let caps = RE_ROUTE_HEADER.captures(line).unwrap();
        println!("{:?}", caps);

        let line = "1007-219.0.0.0/9          unicast [ebgp_rs4 10:38:20.602 from 10.255.253.250] * (100) [AS64967i]";
        let caps = RE_ROUTE_HEADER.captures(line).unwrap();
        println!("{:?}", caps);
    }

    #[test]
    fn test_match_route_header_follow() {
        let line =
            "1007-                     unicast [R197_58 2023-04-19 09:28:23] (100) [AS9318i]";
        let caps = RE_ROUTE_HEADER.captures(line).unwrap();
        println!("{:?}", caps.name("from_protocol").unwrap().as_str());
    }

    #[test]
    fn test_parse_route_meta() {
        let mut route = Route::default();
        let line = "    via 172.31.195.39 on vx0";
        let state = parse_route_meta(&mut route, line).unwrap();
        assert_eq!(state, State::Meta);
        let line = "1008-   Type: BGP univ";
        let state = parse_route_meta(&mut route, line).unwrap();
        assert_eq!(state, State::BGP(BGPState::Attributes));

        assert_eq!(route.gateway, "172.31.195.39");
        assert_eq!(route.interface, "vx0");
    }

    #[test]
    fn test_parse_large_communities() {
        let line = "(111, 0, 1120) (222, 0, 123) (333, 222, 333)";
        let communities = parse_large_communities(line).unwrap();

        let expected = vec![
            LargeCommunity(111, 0, 1120),
            LargeCommunity(222, 0, 123),
            LargeCommunity(333, 222, 333),
        ];

        for (i, e) in expected.iter().enumerate() {
            assert_eq!(&communities[i], e);
        }
    }

    #[test]
    fn test_parse_ext_communities() {
        let line = "BGP.ext_community: (rt, 64512, 21) (rt, 64512, 10) (rt, 64512, 41) (generic, 0x43000000, 0x1) (rt, 64512, 52)";
        let communities = parse_ext_communities(line).expect("must parse");

        let expected = vec![
            ExtCommunity("rt".into(), "64512".into(), "21".into()),
            ExtCommunity("rt".into(), "64512".into(), "10".into()),
            ExtCommunity("rt".into(), "64512".into(), "41".into()),
            ExtCommunity("generic".into(), "0x43000000".into(), "0x1".into()),
            ExtCommunity("rt".into(), "64512".into(), "52".into()),
        ];

        for (i, e) in expected.iter().enumerate() {
            assert_eq!(&communities[i], e);
        }
    }

    #[test]
    fn test_parse_route_simple_v4() {
        let block = include_str!("../../tests/birdc/show-route-all-v4");
        let block: Vec<String> =
            block.split("\n").map(|s| s.to_string()).collect();
        let route = Route::parse(block).unwrap();

        assert_eq!(route.bgp.ext_communities.len(), 1);
        let comm = route.bgp.ext_communities[0].clone();
        assert_eq!(
            comm,
            ExtCommunity("rt".into(), "271042".into(), "0".into())
        );

        assert_eq!(route.bgp.communities.len(), 65);
        assert_eq!(route.bgp.large_communities.len(), 3);
    }

    #[test]
    fn test_parse_route_w_aggregator() {
        let block =
            include_str!("../../tests/birdc/show-route-all-bgp-aggregator");
        let block: Vec<String> =
            block.split("\n").map(|s| s.to_string()).collect();
        let route = Route::parse(block).unwrap();

        assert_eq!(route.bgp.large_communities.len(), 2);
        assert_eq!(route.bgp.otc, Some("213973".into()));
    }
}
