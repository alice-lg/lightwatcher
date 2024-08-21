use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    parsers::{
        datetime,
        parser::{Block, BlockGroup, Parse},
    },
    state::{Community, ExtCommunity, LargeCommunity, Route},
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
    static ref RE_BGP_COMMUNITY: Regex = Regex::new(r"\((.+), (\d+), (\d+)\)").unwrap();

    pub static ref RE_ROUTES_START: Regex = Regex::new(r"1007-\S").unwrap();
    static ref RE_ROUTE_START: Regex = Regex::new(r"1007-").unwrap();
}

#[derive(Debug, PartialEq)]
enum CommunityType {
    Standard,
    Extended,
    Large,
}

/// Route Parser State
#[derive(Debug, PartialEq)]
enum State {
    Start,
    Meta,
    BGP,
    Communities(CommunityType),
}

/// A routes group that shares the same prefix. However while parsing
/// only the first route has a prefix.
pub type PrefixGroup = Vec<Route>;

impl Parse for PrefixGroup {
    fn parse(block: Block) -> Result<Self> {
        let mut routes: PrefixGroup = Vec::new();
        let mut iter = BlockGroup::new(block, &RE_ROUTE_START);
        let mut prefix: String = String::new(); // Current prefix

        while let Some(block) = iter.next() {
            if block[0].starts_with("0001") {
                continue;
            }
            let mut route = Route::parse(block)?;
            if route.network == "" {
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
impl Parse for Route {
    fn parse(block: Block) -> Result<Self> {
        let mut route = Route::default();
        let mut state = State::Start;
        for line in block.iter() {
            match parse_line(&mut route, state, &line) {
                Ok(next_state) => state = next_state,
                Err(e) => {
                    println!("Error parsing line: {}, {}", line, e);
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
        State::BGP => parse_route_bgp(route, line),
        State::Communities(community_type) => {
            parse_route_communities(route, community_type, line)
        }
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
            route.age = datetime::parse_duration_sec(age.as_str())?;
        }
        if let Some(_) = caps.name("primary") {
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

    Ok(State::BGP)
}

/// Parse AS path
fn parse_as_path(s: &str) -> Result<Vec<u32>> {
    let mut as_path: Vec<u32> = vec![];
    for asn in s.split(" ") {
        as_path.push(asn.parse()?);
    }
    Ok(as_path)
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

/// Parse a list of ext communities
fn parse_ext_communities(s: &str) -> Result<Vec<ExtCommunity>> {
    let communities: Vec<ExtCommunity> = RE_BGP_COMMUNITY
        .captures_iter(s)
        .map(|c| {
            ExtCommunity(
                c[1].to_string(),
                c[2].parse().unwrap_or(0),
                c[3].parse().unwrap_or(0),
            )
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
                c[1].parse().unwrap_or(0),
                c[2].parse().unwrap_or(0),
                c[3].parse().unwrap_or(0),
            )
        })
        .collect();
    Ok(communities)
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

fn parse_route_communities(
    route: &mut Route,
    community_type: CommunityType,
    line: &str,
) -> Result<State> {
    let mut line = line.trim_start();
    let next_type = if line.starts_with("BGP.community") {
        CommunityType::Standard
    } else if line.starts_with("BGP.large_community") {
        CommunityType::Large
    } else if line.starts_with("BGP.ext_community") {
        CommunityType::Extended
    } else {
        community_type
    };

    // Strip everything before the colon
    if let Some(index) = line.find(':') {
        line = &line[index + 1..];
    }
    line = line.trim_start();
    if line.is_empty() {
        return Ok(State::Communities(next_type));
    }

    // Append to existing list of communities
    match next_type {
        CommunityType::Standard => {
            route
                .bgp
                .communities
                .append(&mut parse_list(line, parse_community)?);
        }
        CommunityType::Large => {
            route
                .bgp
                .large_communities
                .append(&mut parse_large_communities(line)?);
        }
        CommunityType::Extended => {
            route
                .bgp
                .ext_communities
                .append(&mut parse_ext_communities(line)?);
        }
    }

    Ok(State::Communities(next_type))
}

/// Parse route BGP
fn parse_route_bgp(route: &mut Route, line: &str) -> Result<State> {
    // Parse key value info
    if let Some(caps) = RE_KEY_VALUE.captures(line) {
        let key = caps["key"].to_lowercase();
        let val = caps["value"].to_string();

        if key == "bgp.origin" {
            route.bgp.origin = val;
        } else if key == "bgp.as_path" {
            route.bgp.as_path = parse_as_path(&val)?;
        } else if key == "bgp.next_hop" {
            route.bgp.next_hop = val;
        } else if key == "bgp.med" {
            route.bgp.med = val.parse()?;
        } else if key == "bgp.local_pref" {
            route.bgp.local_pref = val.parse()?;
            // After this the Communities start
            return Ok(State::Communities(CommunityType::Standard));
        }
    }

    Ok(State::BGP)
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
        assert_eq!(state, State::BGP);

        assert_eq!(route.gateway, "172.31.195.39");
        assert_eq!(route.interface, "vx0");
    }

    #[test]
    fn test_parse_large_communities() {
        let line = "(57463, 0, 1120) (57463, 0, 5408) (57463, 0, 6461)";
        let communities = parse_large_communities(line).unwrap();
        println!("{:?}", communities);
    }

    #[test]
    fn test_parse_route() {
        let block = r#"1007-196.216.248.0/23     unicast [R193_103 2023-04-19 09:35:57] * (100) [AS37062i]
 	via 172.31.193.103 on vx0
1008-	Type: BGP univ
1012-	BGP.origin: IGP
 	BGP.as_path: 37468 328154 37062 37062 37062
 	BGP.next_hop: 172.31.193.103
 	BGP.local_pref: 100
 	BGP.community: (0,2854) (0,3216) (0,5580) (0,6939) (0,8075) (0,8359) (0,8641) (0,8732) (0,8752) (0,9009) (0,12494) (0,12695) (0,12714) (0,13335) (0,15672) (0,15895) (0,16509) (0,20485) (0,20764) (0,20940) (0,28917) (0,29049) (0,29076) (0,30833) (0,31059) (0,31133) (0,31261) (0,31500) (0,32590) (0,35598) (0,41268) (0,41275) (0,42861) (0,43727) (0,44053) (0,44843) (0,47541) (0,47542) (0,47626) (0,47764) (0,48166) (0,48293) (0,48719) (0,48858) (0,49505) (0,49830) (0,49981) (0,50923)
 		(0,51028) (0,51343) (0,51907) (0,52091) (0,56630) (0,59624) (0,59796) (0,60280) (0,60764) (9198,5803) (9198,5990) (9198,58031) (65002,20940) (65101,1085) (65102,1000) (65103,276) (65104,150)
 	BGP.ext_community: (rt, 271042, 0)
 	BGP.large_community: (6695, 1000, 1) (57463, 0, 5408) (57463, 0, 6461)
            "#;
        let block: Vec<String> =
            block.split("\n").map(|s| s.to_string()).collect();
        let route = Route::parse(block).unwrap();

        println!("{:?}", route);
    }

    /*
    #[test]
    fn test_routes_reader() {
        // let file: File = File::open("tests/birdc/show-route-all-protocol-R192_175").unwrap();
        let file: File = File::open("tests/birdc/show-route-all-table-master4").unwrap();
        let reader = BufReader::new(file);
        let mut reader = RoutesReader::new(reader);
        let routes: Vec<Route> = reader.collect();
        println!("Decoded {:?}", routes.len());
        println!("{:?}", routes[5]);
    }
    */
}
