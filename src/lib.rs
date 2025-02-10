//! SOCKS proxy clients

#![deny(clippy::all)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![warn(missing_docs)]

use std::{
    fmt::Formatter,
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
    vec,
};
#[cfg(feature = "client")]
pub use v4::client::Socks4Stream;
#[cfg(feature = "client")]
pub use v5::client::Socks5Stream;

#[cfg(feature = "bind")]
pub use v4::bind::Socks4Listener;
#[cfg(feature = "bind")]
pub use v5::bind::Socks5Listener;

#[cfg(feature = "udp")]
pub use v5::udp::Socks5Datagram;

pub use error::{unwrap_io_to_socks2_error, Error};

mod error;
#[cfg(feature = "udp")]
mod io_ext;
#[cfg(any(feature = "client", feature = "bind"))]
mod v4;
#[cfg(any(feature = "client", feature = "bind", feature = "udp"))]
mod v5;

/// A description of a connection target.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TargetAddr {
    /// Connect to an IP address.
    Ip(SocketAddr),
    /// Connect to a fully qualified domain name.
    ///
    /// The domain name will be passed along to the proxy server and DNS lookup
    /// will happen there.
    Domain(String, u16),
}

impl std::fmt::Display for TargetAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ip(addr) => write!(f, "{addr}"),
            Self::Domain(domain, port) => write!(f, "{domain}:{port}"),
        }
    }
}

impl ToSocketAddrs for TargetAddr {
    type Iter = Iter;

    fn to_socket_addrs(&self) -> io::Result<Iter> {
        let inner = match *self {
            Self::Ip(addr) => IterInner::Ip(Some(addr)),
            Self::Domain(ref domain, port) => {
                let it = (&**domain, port).to_socket_addrs()?;
                IterInner::Domain(it)
            }
        };
        Ok(Iter(inner))
    }
}

enum IterInner {
    Ip(Option<SocketAddr>),
    Domain(vec::IntoIter<SocketAddr>),
}

/// An iterator over `SocketAddr`s associated with a `TargetAddr`.
pub struct Iter(IterInner);

impl Iterator for Iter {
    type Item = SocketAddr;

    fn next(&mut self) -> Option<SocketAddr> {
        match self.0 {
            IterInner::Ip(ref mut addr) => addr.take(),
            IterInner::Domain(ref mut it) => it.next(),
        }
    }
}

/// A trait for objects that can be converted to `TargetAddr`.
pub trait ToTargetAddr {
    /// Converts the value of `self` to a `TargetAddr`.
    ///
    /// # Errors
    /// - `std::io::ErrorKind::*`
    fn to_target_addr(&self) -> io::Result<TargetAddr>;
}

impl ToTargetAddr for TargetAddr {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        Ok(self.clone())
    }
}

impl ToTargetAddr for SocketAddr {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        Ok(TargetAddr::Ip(*self))
    }
}

impl ToTargetAddr for SocketAddrV4 {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddr::V4(*self).to_target_addr()
    }
}

impl ToTargetAddr for SocketAddrV6 {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddr::V6(*self).to_target_addr()
    }
}

impl ToTargetAddr for (Ipv4Addr, u16) {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddrV4::new(self.0, self.1).to_target_addr()
    }
}

impl ToTargetAddr for (Ipv6Addr, u16) {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        SocketAddrV6::new(self.0, self.1, 0, 0).to_target_addr()
    }
}

impl ToTargetAddr for (&str, u16) {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        // try to parse as an IP first
        if let Ok(addr) = self.0.parse::<Ipv4Addr>() {
            return (addr, self.1).to_target_addr();
        }

        if let Ok(addr) = self.0.parse::<Ipv6Addr>() {
            return (addr, self.1).to_target_addr();
        }

        Ok(TargetAddr::Domain(self.0.to_owned(), self.1))
    }
}

impl ToTargetAddr for &str {
    fn to_target_addr(&self) -> io::Result<TargetAddr> {
        // try to parse as an IP first
        if let Ok(addr) = self.parse::<SocketAddrV4>() {
            return addr.to_target_addr();
        }

        if let Ok(addr) = self.parse::<SocketAddrV6>() {
            return addr.to_target_addr();
        }

        // split the string by ':' and convert the second part to u16
        let mut parts_iter = self.rsplitn(2, ':');
        let Some(port_str) = parts_iter.next() else {
            return Err(Error::InvalidSocksAddress {
                addr: (*self).to_string(),
            }
            .into_io());
        };

        let Some(host) = parts_iter.next() else {
            return Err(Error::InvalidSocksAddress {
                addr: (*self).to_string(),
            }
            .into_io());
        };

        let Some(port): Option<u16> = port_str.parse().ok() else {
            return Err(Error::InvalidPortValue {
                addr: (*self).to_string(),
                port: port_str.to_string(),
            }
            .into_io());
        };

        (host, port).to_target_addr()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    use super::*;

    #[test]
    fn domains_to_target_addr() {
        assert_eq!(
            "localhost:80".to_target_addr().unwrap(),
            TargetAddr::Domain("localhost".to_owned(), 80)
        );
        assert_eq!(
            unwrap_io_to_socks2_error(&"localhost:".to_target_addr().unwrap_err()),
            Some(&Error::InvalidPortValue {
                addr: String::new(),
                port: String::new()
            })
        );
        assert_eq!(
            "github.com:443".to_target_addr().unwrap(),
            TargetAddr::Domain("github.com".to_owned(), 443)
        );
    }
}
