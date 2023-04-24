use anyhow::Result;
use std::io::{BufRead, BufReader, Read};

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
pub trait Reader<T> {
    fn read<R: Read>(reader: BufReader<R>) -> Result<T>;
}

pub struct BlockIterator<R: Read> {
    reader: BufReader<R>,
}

impl<R: Read> BlockIterator<R> {
    pub fn new(reader: BufReader<R>) -> Self {
        Self { reader }
    }
}

impl<R: Read> Iterator for BlockIterator<R> {
    type Item = Block;

    fn next(&mut self) -> Option<Self::Item> {
        let mut block = Block::new();
        let mut line = String::new();
        let mut total_read = 0;

        loop {
            let bytes_read = self.reader.read_line(&mut line).ok()?;
            if bytes_read == 0 {
                break;
            }
            total_read += bytes_read;

            line = line.trim_end().to_string();
            if line.is_empty() {
                break;
            } else {
                block.push(line.clone());
            }
            line.clear();
        }

        if total_read == 0 {
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
        let input = "line1\nline2\n\nline3\n\nline4\n\n\nline5\n";
        let reader = BufReader::new(input.as_bytes());
        let mut iter = BlockIterator { reader };

        assert_eq!(iter.next().unwrap(), vec!["line1", "line2"]);
        assert_eq!(iter.next().unwrap(), vec!["line3"]);
        assert_eq!(iter.next().unwrap(), vec!["line4"]);
        assert_eq!(iter.next().unwrap(), Block::new());
        assert_eq!(iter.next().unwrap(), vec!["line5"]);
        assert_eq!(iter.next(), None);
    }
}
