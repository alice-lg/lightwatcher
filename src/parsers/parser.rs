use anyhow::Result;
use regex::Regex;
use std::io::{BufRead, Lines};
use std::iter::Peekable;

#[derive(thiserror::Error, Debug)]
pub struct ParseError {
    line: String,
    #[source]
    source: anyhow::Error,
}

impl ParseError {
    pub fn new(line: String, source: anyhow::Error) -> Self {
        Self { line, source }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error in line '{}': {}", self.line, self.source)
    }
}

/// A block is a list of lines
pub type Block = Vec<String>;

/// Parse is a parser trait which can be implemented
pub trait Parse<T>: Sized {
    fn parse(input: T) -> Result<Self>;
}

/// A block group iterates a block and emits new blocks when
/// a starting condition matches.
pub struct BlockGroup {
    iter: Peekable<std::vec::IntoIter<String>>,
    start: Regex,
}

impl BlockGroup {
    /// Create a new iterable block group
    pub fn new(block: Block, start: &Regex) -> Self {
        Self {
            iter: block.into_iter().peekable(),
            start: start.clone(),
        }
    }
}

/// Implement iterator for block group.
/// This is pretty much the same as the block iterator, so
/// maybe these two can be merged into one. However, my rust
/// skills are not good enough to do this.
impl Iterator for BlockGroup {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let mut block = Block::new();

        // Create a peekable line iterator
        loop {
            // Read next line
            let line = self.iter.next()?;
            // Add line to block
            block.push(line.clone());

            // Check next line in iterator
            if let Some(next) = self.iter.peek() {
                if self.start.is_match(next) {
                    break;
                }
            } else {
                break;
            }
        }

        if block.is_empty() {
            None
        } else {
            Some(block)
        }
    }
}

/// A BlockIterator takes an object implementing the Read trait
/// and a marker token
/// which separates the input lines into blocks.
/// A new block starts when the marker token is found.
pub struct BlockIterator<R: BufRead> {
    start: Regex,
    stop: Option<Regex>,
    lines: Peekable<Lines<R>>,
}

impl<R: BufRead> BlockIterator<R> {
    /// Create a new BlockIterator
    pub fn new(reader: R, start: &Regex) -> Self {
        Self {
            start: start.clone(),
            stop: None,
            lines: reader.lines().peekable(),
        }
    }

    /// Configure a stop condition
    pub fn with_stop(self, stop: &Regex) -> Self {
        Self {
            start: self.start,
            stop: Some(stop.clone()),
            lines: self.lines,
        }
    }
}

/// Implement the Iterator trait for BlockIterator
impl<R: BufRead> Iterator for BlockIterator<R> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let mut block = Block::new();

        // Create a peekable line iterator
        loop {
            // Read next line
            let line = self.lines.next()?.ok()?;

            // Check stop marker
            if line.starts_with("0000") {
                return None;
            }

            if line.starts_with("9001") {
                tracing::error!(line = line, "error when parsing");
                return None;
            }

            block.push(line.clone());

            // Check if we run into a soft stop
            if let Some(re) = &self.stop {
                if re.is_match(&line) {
                    break;
                }
            }

            // Check next line in iterator
            if let Some(Ok(next)) = self.lines.peek() {
                if self.start.is_match(next) {
                    break;
                }
                if next.starts_with("0000") {
                    break;
                }
            } else {
                break;
            }
        }

        if block.is_empty() {
            None
        } else {
            Some(block)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    #[test]
    fn test_block_iterator() {
        let input = "1003-line1\nline2\n\n1003-line3\n\nline4\n9009-line5\n";
        let reader = BufReader::new(input.as_bytes());
        let re_start = Regex::new(r"1003-").unwrap();
        let mut iter = BlockIterator::new(reader, &re_start);

        assert_eq!(iter.next().unwrap(), vec!["1003-line1", "line2", ""]);
        assert_eq!(
            iter.next().unwrap(),
            vec!["1003-line3", "", "line4", "9009-line5"]
        );
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_block_group_iterator() {
        let block = vec![
            "1003-line1".to_string(),
            "line2".to_string(),
            "".to_string(),
            "1003-line3".to_string(),
            "".to_string(),
            "line4".to_string(),
            "9009-line5".to_string(),
        ];

        let re_start = Regex::new(r"1003-").unwrap();
        let mut iter = BlockGroup::new(block, &re_start);

        assert_eq!(iter.next().unwrap(), vec!["1003-line1", "line2", ""]);
        assert_eq!(
            iter.next().unwrap(),
            vec!["1003-line3", "", "line4", "9009-line5"]
        );
        assert_eq!(iter.next(), None);
    }
}
