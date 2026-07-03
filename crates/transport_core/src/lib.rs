//! Transport trait + BufferPool contract for Polaris networking stack

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Greeter {
    fn greet(&self, name: &str) -> Result<String>;
}

pub struct DefaultGreeter;

impl Greeter for DefaultGreeter {
    fn greet(&self, name: &str) -> Result<String> {
        if name.is_empty() {
            return Err(Error::InvalidInput("name is empty".into()));
        }
        Ok(format!("hello, {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greet_returns_hello() {
        let g = DefaultGreeter;
        assert_eq!(g.greet("world").unwrap(), "hello, world");
    }

    #[test]
    fn greet_rejects_empty_name() {
        let g = DefaultGreeter;
        assert!(matches!(g.greet(""), Err(Error::InvalidInput(_))));
    }
}
