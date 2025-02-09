// TODO: Replace all string errors with this!

use std::{fmt::Formatter, io};

/// Errors from socks2
#[derive(Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    // TODO: Add docs for all errors?
    InvalidSocksAddress { addr: String },
    InvalidPortValue { addr: String, port: String },
    WinUDP4GiBLimit { size: usize },
}

// impl Error {
//     pub(crate) fn into_io(self) -> io::Error {
//         self.into()
//     }
// }

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        macro_rules! peq {
            ($($s:ident),+) => {
                match (self, other) {
                    $((Self::$s { .. }, Self::$s { .. }) => true,)+
                    _ => false,
                }
            };
        }

        peq!(InvalidSocksAddress, InvalidPortValue, WinUDP4GiBLimit)
    }
}

impl From<Error> for io::Error {
    fn from(value: Error) -> Self {
        macro_rules! from_error {
            ($(($s:ident, $k:ident)),+) => {
                match value {
                    $(Error::$s { .. } => io::Error::new(io::ErrorKind::$k, value),)+
                }
            };
        }

        from_error!(
            (InvalidSocksAddress, InvalidInput),
            (InvalidPortValue, InvalidInput),
            (WinUDP4GiBLimit, InvalidInput)
        )
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSocksAddress { addr } => write!(f, "invalid socket address '{addr}'"),
            Self::InvalidPortValue { addr, port } => {
                write!(f, "invalid port value '{port}' for '{addr}'")
            },
            Self::WinUDP4GiBLimit {size} => write!(f, "tried to write '{size}' bytes to UDPSocket, but writev/readv has a 4 GiB limit on windows"),
        }
    }
}
