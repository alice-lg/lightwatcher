use anyhow::Result;
use std::io::{BufRead, BufReader, Lines, Read};
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

/// A parser is a function that takes a chunk and
/// parses it into a type T.
pub trait Parser<T> {
    fn parse(block: Block) -> Result<T>;
}

/// A Reader is a trait that can be used to
/// parse a collection of a type T.
pub trait Reader {
    type Item;
    fn read<R: Read>(reader: BufReader<R>) -> Result<Self::Item>;
}

/// A BlockIterator takes an object implementing the Read trait
/// and a marker token
/// which separates the input lines into blocks.
/// A new block starts when the marker token is found.
pub struct BlockIterator<R: BufRead> {
    start: String,
    lines: Peekable<Lines<R>>,
}

impl<R: BufRead> BlockIterator<R> {
    /// Create a new BlockIterator
    pub fn new(reader: R, start: &str) -> Self {
        Self {
            start: start.to_string(),
            lines: reader.lines().peekable(),
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
            let line = self.lines.next()?;
            // Add line to block
            let line = line.ok()?;
            block.push(line.clone());

            // Check next line in iterator
            if let Some(Ok(next)) = self.lines.peek() {
                if next.starts_with(&self.start) {
                    break;
                }
            } else {
                break; // EOF
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

    #[test]
    fn test_block_iterator() {
        let input = "1003-line1\nline2\n\n1003-line3\n\nline4\n9009-line5\n";
        let reader = BufReader::new(input.as_bytes());
        let mut iter = BlockIterator::new(reader, "1003-");

        assert_eq!(iter.next().unwrap(), vec!["1003-line1", "line2", ""]);
        assert_eq!(
            iter.next().unwrap(),
            vec!["1003-line3", "", "line4", "9009-line5"]
        );
        assert_eq!(iter.next(), None);
    }
}
