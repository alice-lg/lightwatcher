use anyhow::Result;
use std::io::{BufRead, Lines};
use std::iter::Peekable;
use std::sync::Arc;

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

pub type ParBlock = Arc<Block>;

/// Parse is a parser trait which can be implemented
pub trait Parse: Sized {
    fn parse(block: Block) -> Result<Self>;
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

/// A ParBlockIterator takes an object implementing the Read trait
/// and a marker token
/// which separates the input lines into blocks.
/// A new block starts when the marker token is found.
pub struct ParBlockIterator<R: BufRead> {
    start: String,
    lines: Peekable<Lines<R>>,
}

impl<R: BufRead> ParBlockIterator<R> {
    /// Create a new BlockIterator
    pub fn new(reader: R, start: &str) -> Self {
        Self {
            start: start.to_string(),
            lines: reader.lines().peekable(),
        }
    }
}

/// Implement the Iterator trait for BlockIterator
impl<R: BufRead> Iterator for ParBlockIterator<R> {
    type Item = ParBlock;

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
            Some(ParBlock::new(block))
        }
    }
}

/*
pub struct Reader<B: BufRead, T> {
    iter: BlockIterator<B>,
}

impl<B, T> Reader<B, T> {
    pub fn new(buf: B, start: &str) -> Self {
        Self {
            iter: BlockIterator::new(buf, start),
        }
    }
}

impl<B, T> Iterator for Reader<B, T>
where
    B: BufRead,
    T: Parse + Default,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let block = self.iter.next()?;
        match T::parse(block) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("Error: {}", e);
                T::default()
            }
        }
        Some(T::parse(block))
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

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
