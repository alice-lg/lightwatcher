use std::io::BufRead;

use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    // Matches: "123 of 456 routes ..." -> groups: (123, 456)
    static ref RE_ROUTES_COUNT: Regex = Regex::new(r"^\d+-(\d+)\s+of\s+(\d+)\s+routes.*$").unwrap();
}

/// Parse routes count from BIRD output
/// TODO: Evaluate. For now this seems to be sufficient.
pub fn parse_routes_count<R: BufRead>(reader: R) -> Result<u32> {
    for line in reader.lines() {
        let line = line?;

        // Skip any other than the results line
        if !line.starts_with("1007-") {
            continue;
        }

        // Match count pattern
        if let Some(caps) = RE_ROUTES_COUNT.captures(&line)
            && let Some(count_str) = caps.get(1)
            && let Ok(count) = count_str.as_str().parse::<u32>()
        {
            return Ok(count);
        }
    }

    Err(anyhow::anyhow!("result did not include routes count"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_routes_count() {
        let data = include_str!("../../tests/birdc/show-route-count");
        let result = parse_routes_count(Cursor::new(data)).unwrap();

        // Expected: 18 of 79308 routes for...
        assert_eq!(result, 18);
    }

    #[test]
    fn test_parse_routes_count_fail() {
        let data =
            Cursor::new("9001 syntax error, unexpected CF_SYM_UNDEFINED,");
        let result = parse_routes_count(data);
        assert!(result.is_err());
    }
}
