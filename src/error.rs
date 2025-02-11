use alloc::string::FromUtf8Error;
use std::{io, net::SocketAddrV6};

/// Errors from socks2
///
/// # Notes
/// `Error` implements `PartialEq`, but it does not compare fields.
#[derive(Debug)]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    // TargetAddr
    /// Domain name and port could not be parsed.
    InvalidSocksAddress { addr: String },
    /// Port could not be parsed or is over `u16::MAX`.
    InvalidPortValue { addr: String, port: String },

    // Socks4/Socks5
    /// Could not resolve any of the socket address.
    NoResolveSocketAddrs {},
    /// Response from server had an invalid version byte.
    InvalidResponseVersion { version: u8 },
    /// Unknown response code
    UnknownResponseCode { code: u8 },
    /// Connection refused or the request was rejected or failed.
    ConnectionRefused { code: u8 },

    // Socks4
    /// Rejected request due to server not connecting to idnetd on the client.
    /// Rejected request due to idnetd and client program not having a matching userid.
    RejectedRequestID { code: u8 },
    /// Socks4 does not support IPv6.
    Socks4NoIPv6 { addr: SocketAddrV6 },

    // Socks5
    /// Domain received from server was not valid utf8.
    MalformedDomain { err: FromUtf8Error },
    /// Received an invalid address type from the server.
    SOCKS5InvalidAddressType { code: u8 },
    /// Unknown error from the server.
    UnknownServerFailure { code: u8 },
    /// Server ruleset does not allow connection.
    ServerRefusedByRuleSet {},
    /// Network unreachable.
    ServerNetworkUnreachable {},
    /// Host unreachable.
    ServerHostUnreachable {},
    /// Time to live expired.
    ServerTTLExpired {},
    /// Server does not support the sent command.
    ServerCmdNotSupported {},
    /// Server does not support the address kind.
    ServerAddressNotSupported {},
    /// Reserved byte from server is invalid.
    InvalidReservedByte { byte: u8 },
    /// Domains must have a length between 1 and 255 inclusive.
    InvalidDomainLength { domain: String, length: usize },
    /// No acceptable auth methods.
    NoAuthMethods { method: u8 },
    /// Unknown auth method.
    UnknownAuthMethod { method: u8 },
    /// Invalid username.
    InvalidUsername { username: String, length: usize },
    /// Invalid password.
    InvalidPassword { password: (), length: usize },
    /// Auth with password failed.
    FailedPasswordAuth {},

    // UDP
    /// Reserved bytes from server is invalid.
    InvalidReservedBytes { bytes: u16 },
    /// Fragment id from the server is invalid.
    InvalidFragmentID { fid: u8 },
    /// UDP Bind Client has a limit of 4 GiB for buffers.
    /// Only occurs when using `Socks5Datagram` on windows.
    WinUDP4GiBLimit { size: usize },
}

/// Takes an `std::io::Error` and attempts to unwrap it into a `socks2::Error`.
/// Returns a `Some(&socks2::Error)` on success.
#[inline]
#[must_use]
pub fn unwrap_io_to_socks2_error(e: &io::Error) -> Option<&Error> {
    e.get_ref().and_then(|i| i.downcast_ref())
}

/// Takes an `std::io::Error` and determines if it wraps a `socks2::Error`;
#[inline]
#[must_use]
pub fn is_io_socks2_error(e: &io::Error) -> bool {
    unwrap_io_to_socks2_error(e).is_some()
}

impl Error {
    #[inline]
    pub(crate) fn into_io(self) -> io::Error {
        self.into()
    }
}

#[cfg(test)]
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

        peq!(
            InvalidSocksAddress,
            InvalidPortValue,
            NoResolveSocketAddrs,
            InvalidResponseVersion,
            UnknownResponseCode,
            ConnectionRefused,
            RejectedRequestID,
            Socks4NoIPv6,
            MalformedDomain,
            SOCKS5InvalidAddressType,
            UnknownServerFailure,
            ServerRefusedByRuleSet,
            ServerNetworkUnreachable,
            ServerHostUnreachable,
            ServerTTLExpired,
            ServerCmdNotSupported,
            ServerAddressNotSupported,
            InvalidReservedByte,
            InvalidDomainLength,
            NoAuthMethods,
            UnknownAuthMethod,
            InvalidUsername,
            InvalidPassword,
            FailedPasswordAuth,
            InvalidReservedBytes,
            InvalidFragmentID,
            WinUDP4GiBLimit
        )
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
            (NoResolveSocketAddrs, InvalidInput),
            (InvalidResponseVersion, InvalidData),
            (UnknownResponseCode, Other),
            (ConnectionRefused, ConnectionRefused),
            (RejectedRequestID, PermissionDenied),
            (Socks4NoIPv6, InvalidInput),
            (MalformedDomain, InvalidData),
            (SOCKS5InvalidAddressType, InvalidData),
            (UnknownServerFailure, Other),
            (ServerRefusedByRuleSet, ConnectionRefused),
            (ServerNetworkUnreachable, ConnectionAborted),
            (ServerHostUnreachable, ConnectionAborted),
            (ServerTTLExpired, Interrupted),
            (ServerCmdNotSupported, Unsupported),
            (ServerAddressNotSupported, Unsupported),
            (InvalidReservedByte, Other),
            (InvalidDomainLength, InvalidInput),
            (NoAuthMethods, Unsupported),
            (UnknownAuthMethod, Unsupported),
            (InvalidUsername, InvalidInput),
            (InvalidPassword, InvalidInput),
            (FailedPasswordAuth, PermissionDenied),
            (InvalidReservedBytes, InvalidData),
            (InvalidFragmentID, InvalidData),
            (WinUDP4GiBLimit, InvalidInput)
        )
    }
}

impl std::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSocksAddress { addr } => write!(f, "invalid socket address '{addr}'"),
            Self::InvalidPortValue { addr, port } => {
                write!(f, "invalid port value '{port}' for '{addr}'")
            },
            Self::NoResolveSocketAddrs {} => write!(f, "could not resolve a socket address"),
            Self::InvalidResponseVersion { version } => write!(f, "invalid response version '{version}'"),
            Self::UnknownResponseCode { code } => write!(f, "unknown response code '{code}'"),
            Self::ConnectionRefused { code } => write!(f, "connection refused or the request was rejected or failed '{code}'"),
            Self::RejectedRequestID { code } => write!(f, "request rejected because of idnetd with code '{code}'"),
            Self::Socks4NoIPv6 { addr } => write!(f, "SOCKS4 does not support IPv6 '{addr}'"),
            Self::MalformedDomain { err } => write!(f, "malformed domain {err}"),
            Self::SOCKS5InvalidAddressType { code } => write!(f, "invalid address type {code}"),
            Self::UnknownServerFailure { code } => write!(f, "unknown server failure {code}"),
            Self::ServerRefusedByRuleSet {} => write!(f, "connection not allowed by ruleset"),
            Self::ServerNetworkUnreachable {} => write!(f, "network unreachable"),
            Self::ServerHostUnreachable {} => write!(f, "host unreachable"),
            Self::ServerTTLExpired {} => write!(f, "TTL expired"),
            Self::ServerCmdNotSupported {} => write!(f, "command not supported"),
            Self::ServerAddressNotSupported {} => write!(f, "address kind not supported"),
            Self::InvalidReservedByte { byte } => write!(f, "invalid reserved byte '{byte}'"),
            Self::InvalidDomainLength { domain, length } => write!(f, "domain '{domain}' with length '{length}' is not between 1-255 inclusive"),
            Self::NoAuthMethods { method } => write!(f, "no acceptable authentication methods '{method}'"),
            Self::UnknownAuthMethod { method } => write!(f, "unknown authentication method '{method}'"),
            Self::InvalidUsername {username, length} => write!(f, "invalid username '{username}' with length '{length}'"),
            Self::InvalidPassword {password, length} => write!(f, "invalid password '{password:?}' with length '{length}'"),
            Self::FailedPasswordAuth {} => write!(f, "password authentication failed"),
            Self::InvalidReservedBytes { bytes } => write!(f, "invalid reserved bytes '{bytes}'"),
            Self::InvalidFragmentID {fid} => write!(f, "invalid fragment ID '{fid}'"),
            Self::WinUDP4GiBLimit {size} => write!(f, "tried to write '{size}' bytes to UDPSocket, but writev/readv has a 4 GiB limit on windows"),
        }
    }
}
