use crate::{Error, TargetAddr};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{
        Read, Write, {self},
    },
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream},
};

const MAX_ADDR_LEN: usize = 260;

fn read_addr<R: Read>(socket: &mut R) -> io::Result<TargetAddr> {
    match socket.read_u8()? {
        1 => {
            let ip = Ipv4Addr::from(socket.read_u32::<BigEndian>()?);
            let port = socket.read_u16::<BigEndian>()?;
            Ok(TargetAddr::Ip(SocketAddr::V4(SocketAddrV4::new(ip, port))))
        }
        3 => {
            let len = socket.read_u8()?;
            let mut domain = vec![0; len as usize];
            socket.read_exact(&mut domain)?;
            let domain = String::from_utf8(domain)
                .map_err(|err| Error::MalformedDomain { err }.into_io())?;
            let port = socket.read_u16::<BigEndian>()?;
            Ok(TargetAddr::Domain(domain, port))
        }
        4 => {
            let mut ip = [0; 16];
            socket.read_exact(&mut ip)?;
            let ip = Ipv6Addr::from(ip);
            let port = socket.read_u16::<BigEndian>()?;
            Ok(TargetAddr::Ip(SocketAddr::V6(SocketAddrV6::new(
                ip, port, 0, 0,
            ))))
        }
        code => Err(Error::SOCKS5InvalidAddressType { code }.into_io()),
    }
}

fn read_response(socket: &mut TcpStream) -> io::Result<TargetAddr> {
    {
        let version = socket.read_u8()?;
        if version != 5 {
            return Err(Error::InvalidResponseVersion { version }.into_io());
        }
    }

    match socket.read_u8()? {
        0 => {}
        1 => return Err(Error::UnknownServerFailure { code: 1 }.into_io()),
        2 => return Err(Error::ServerRefusedByRuleSet {}.into_io()),
        3 => return Err(Error::ServerNetworkUnreachable {}.into_io()),
        4 => return Err(Error::ServerHostUnreachable {}.into_io()),
        5 => return Err(Error::ConnectionRefused { code: 5 }.into_io()),
        6 => return Err(Error::ServerTTLExpired {}.into_io()),
        7 => return Err(Error::ServerCmdNotSupported {}.into_io()),
        8 => return Err(Error::ServerAddressNotSupported {}.into_io()),
        code => return Err(Error::UnknownServerFailure { code }.into_io()),
    }

    {
        let byte = socket.read_u8()?;
        if byte != 0 {
            return Err(Error::InvalidReservedByte { byte }.into_io());
        }
    }

    read_addr(socket)
}

fn write_addr(mut packet: &mut [u8], target: &TargetAddr) -> io::Result<usize> {
    let start_len = packet.len();
    match *target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            packet.write_u8(1)?;
            packet.write_u32::<BigEndian>((*addr.ip()).into())?;
            packet.write_u16::<BigEndian>(addr.port())?;
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            packet.write_u8(4)?;
            packet.write_all(&addr.ip().octets())?;
            packet.write_u16::<BigEndian>(addr.port())?;
        }
        TargetAddr::Domain(ref domain, port) => {
            packet.write_u8(3)?;
            let Some(domain_len) =
                u8::try_from(domain.len())
                    .ok()
                    .and_then(|i| if i == 0 { None } else { Some(i) })
            else {
                return Err(Error::InvalidDomainLength {
                    domain: domain.to_string(),
                    length: domain.len(),
                }
                .into_io());
            };
            packet.write_u8(domain_len)?;
            packet.write_all(domain.as_bytes())?;
            packet.write_u16::<BigEndian>(port)?;
        }
    }

    Ok(start_len - packet.len())
}

/// Authentication methods
#[derive(Debug)]
enum Authentication<'a> {
    Password {
        username: &'a str,
        password: &'a str,
    },
    None,
}

impl Authentication<'_> {
    const fn id(&self) -> u8 {
        match *self {
            Authentication::Password { .. } => 2,
            Authentication::None => 0,
        }
    }

    const fn is_no_auth(&self) -> bool {
        matches!(*self, Authentication::None)
    }
}

#[cfg(feature = "client")]
pub mod client {
    use crate::{
        tcp_stream_connect,
        v5::{read_response, write_addr, Authentication, MAX_ADDR_LEN},
        Error, TargetAddr, ToTargetAddr,
    };
    use core::time::Duration;
    use std::{
        io,
        io::{Read, Write},
        net::{TcpStream, ToSocketAddrs},
    };

    /// A SOCKS5 and SOCKS5H client.
    #[derive(Debug)]
    pub struct Socks5Stream {
        pub(super) socket: TcpStream,
        pub(super) proxy_addr: TargetAddr,
    }

    impl Socks5Stream {
        /// Connects to a target server through a SOCKS5 proxy.
        ///
        /// # Notes
        /// If `target` is a `TargetAddr::Domain`, the domain name will be forwarded
        /// to the proxy server to be resolved there.
        ///
        /// When using `connect_timeout` the duration will apply to every socket address
        /// tried. Only the last connection error will be returned or
        /// `io::Error(Error::NoResolveSocketAddrs)`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn connect<T, U>(
            proxy: T,
            target: &U,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            Self::connect_raw(1, proxy, target, &Authentication::None, connect_timeout)
        }

        /// Connects to a target server through a SOCKS5 proxy using given
        /// username and password.
        ///
        /// # Notes
        /// See `Socks5Stream::connect()`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn connect_with_password<T, U>(
            proxy: T,
            target: &U,
            username: &str,
            password: &str,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            let auth = Authentication::Password { username, password };
            Self::connect_raw(1, proxy, target, &auth, connect_timeout)
        }

        pub(super) fn connect_raw<T, U>(
            command: u8,
            proxy: T,
            target: &U,
            auth: &Authentication,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            let mut socket = tcp_stream_connect(proxy, connect_timeout)?;

            let target = target.to_target_addr()?;

            let packet_len = if auth.is_no_auth() { 3 } else { 4 };
            let packet = [
                5,                                     // protocol version
                if auth.is_no_auth() { 1 } else { 2 }, // method count
                auth.id(),                             // method
                0,                                     // no auth (always offered)
            ];
            socket.write_all(&packet[..packet_len])?;

            let mut buf = [0; 2];
            socket.read_exact(&mut buf)?;
            let response_version = buf[0];
            let selected_method = buf[1];

            if response_version != 5 {
                return Err(Error::InvalidResponseVersion {
                    version: response_version,
                }
                .into_io());
            }

            if selected_method == 0xff {
                return Err(Error::NoAuthMethods {
                    method: selected_method,
                }
                .into_io());
            }

            if selected_method != auth.id() && selected_method != Authentication::None.id() {
                return Err(Error::UnknownAuthMethod {
                    method: selected_method,
                }
                .into_io());
            }

            match *auth {
                Authentication::Password { username, password } if selected_method == auth.id() => {
                    Self::password_authentication(&mut socket, username, password)?;
                }
                _ => (),
            }

            let mut packet = [0; MAX_ADDR_LEN + 3];
            packet[0] = 5; // protocol version
            packet[1] = command; // command
            packet[2] = 0; // reserved
            let len = write_addr(&mut packet[3..], &target)?;
            socket.write_all(&packet[..len + 3])?;

            let proxy_addr = read_response(&mut socket)?;

            Ok(Self { socket, proxy_addr })
        }

        fn password_authentication(
            socket: &mut TcpStream,
            username: &str,
            password: &str,
        ) -> io::Result<()> {
            let Some(username_len) =
                u8::try_from(username.len())
                    .ok()
                    .and_then(|i| if i == 0 { None } else { Some(i) })
            else {
                return Err(Error::InvalidUsername {
                    username: username.to_string(),
                    length: username.len(),
                }
                .into_io());
            };

            let Some(password_len) =
                u8::try_from(password.len())
                    .ok()
                    .and_then(|i| if i == 0 { None } else { Some(i) })
            else {
                return Err(Error::InvalidPassword {
                    password: (),
                    length: password.len(),
                }
                .into_io());
            };

            let mut packet = [0; 515];
            let packet_size = 3 + username.len() + password.len();
            packet[0] = 1; // version
            packet[1] = username_len;
            packet[2..2 + username.len()].copy_from_slice(username.as_bytes());
            packet[2 + username.len()] = password_len;
            packet[3 + username.len()..packet_size].copy_from_slice(password.as_bytes());
            socket.write_all(&packet[..packet_size])?;

            let mut buf = [0; 2];
            socket.read_exact(&mut buf)?;
            if buf[0] != 1 {
                return Err(Error::InvalidResponseVersion { version: buf[0] }.into_io());
            }
            if buf[1] != 0 {
                return Err(Error::FailedPasswordAuth {}.into_io());
            }

            Ok(())
        }

        /// Returns the proxy-side address of the connection between the proxy and
        /// target server.
        #[must_use]
        pub const fn proxy_addr(&self) -> &TargetAddr {
            &self.proxy_addr
        }

        /// Returns a shared reference to the inner `TcpStream`.
        #[must_use]
        pub const fn get_ref(&self) -> &TcpStream {
            &self.socket
        }

        /// Returns a mutable reference to the inner `TcpStream`.
        pub fn get_mut(&mut self) -> &mut TcpStream {
            &mut self.socket
        }

        /// Consumes the `Socks5Stream`, returning the inner `TcpStream`.
        #[must_use]
        pub fn into_inner(self) -> TcpStream {
            self.socket
        }
    }

    impl Read for Socks5Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.socket.read(buf)
        }
    }

    impl Read for &Socks5Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            (&self.socket).read(buf)
        }
    }

    impl Write for Socks5Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.socket.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.socket.flush()
        }
    }

    impl Write for &Socks5Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            (&self.socket).write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            (&self.socket).flush()
        }
    }
}

#[cfg(feature = "bind")]
pub mod bind {
    use crate::{
        v5::{read_response, Authentication},
        Socks5Stream, TargetAddr, ToTargetAddr,
    };
    use core::time::Duration;
    use std::{io, net::ToSocketAddrs};

    /// A SOCKS5 and SOCKS5H BIND client.
    #[derive(Debug)]
    pub struct Socks5Listener(Socks5Stream);

    impl Socks5Listener {
        /// Initiates a BIND request to the specified proxy.
        ///
        /// The proxy will filter incoming connections based on the value of
        /// `target`.
        ///
        /// # Notes
        /// See `Socks5Stream::connect()`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn bind<T, U>(
            proxy: T,
            target: &U,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            Socks5Stream::connect_raw(2, proxy, target, &Authentication::None, connect_timeout)
                .map(Socks5Listener)
        }
        /// Initiates a BIND request to the specified proxy using given username
        /// and password.
        ///
        /// The proxy will filter incoming connections based on the value of
        /// `target`.
        ///
        /// # Notes
        /// See `Socks5Stream::connect()`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn bind_with_password<T, U>(
            proxy: T,
            target: &U,
            username: &str,
            password: &str,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            let auth = Authentication::Password { username, password };
            Socks5Stream::connect_raw(2, proxy, target, &auth, connect_timeout).map(Socks5Listener)
        }

        /// The address of the proxy-side TCP listener.
        ///
        /// This should be forwarded to the remote process, which should open a
        /// connection to it.
        #[must_use]
        pub const fn proxy_addr(&self) -> &TargetAddr {
            &self.0.proxy_addr
        }

        /// Waits for the remote process to connect to the proxy server.
        ///
        /// The value of `proxy_addr` should be forwarded to the remote process
        /// before this method is called.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn accept(mut self) -> io::Result<Socks5Stream> {
            self.0.proxy_addr = read_response(&mut self.0.socket)?;
            Ok(self.0)
        }
    }
}

#[cfg(feature = "udp")]
pub mod udp {
    use crate::{
        io_ext::IOVecExt,
        v5::{read_addr, write_addr, Authentication, MAX_ADDR_LEN},
        Error, Socks5Stream, TargetAddr, ToTargetAddr,
    };
    use byteorder::{BigEndian, ReadBytesExt};
    use core::{cmp, ptr, time::Duration};
    use std::{
        io,
        net::{Ipv4Addr, SocketAddr, SocketAddrV4, ToSocketAddrs, UdpSocket},
    };

    /// A SOCKS5 and SOCKS5H UDP client.
    #[derive(Debug)]
    pub struct Socks5Datagram {
        socket: UdpSocket,
        // keeps the session alive
        stream: Socks5Stream,
    }

    impl Socks5Datagram {
        /// Creates a UDP socket bound to the specified address which will have its
        /// traffic routed through the specified proxy.
        ///
        /// # Notes
        /// See `Socks5Stream::connect()`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn bind<T, U>(proxy: T, addr: U, connect_timeout: Option<Duration>) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToSocketAddrs,
        {
            Self::bind_internal(proxy, addr, &Authentication::None, connect_timeout)
        }

        /// Creates a UDP socket bound to the specified address which will have its
        /// traffic routed through the specified proxy. The given username and password
        /// is used to authenticate to the SOCKS proxy.
        ///
        /// # Notes
        /// See `Socks5Stream::connect()`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn bind_with_password<T, U>(
            proxy: T,
            addr: U,
            username: &str,
            password: &str,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToSocketAddrs,
        {
            let auth = Authentication::Password { username, password };
            Self::bind_internal(proxy, addr, &auth, connect_timeout)
        }

        fn bind_internal<T, U>(
            proxy: T,
            addr: U,
            auth: &Authentication,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToSocketAddrs,
        {
            // we don't know what our IP is from the perspective of the proxy, so
            // don't try to pass `addr` in here.
            let dst = TargetAddr::Ip(SocketAddr::V4(SocketAddrV4::new(
                Ipv4Addr::new(0, 0, 0, 0),
                0,
            )));
            let stream = Socks5Stream::connect_raw(3, proxy, &dst, auth, connect_timeout)?;

            let socket = UdpSocket::bind(addr)?;
            socket.connect(&stream.proxy_addr)?;

            Ok(Self { socket, stream })
        }

        /// Like `UdpSocket::send_to`.
        ///
        /// # Note
        /// The SOCKS protocol inserts a header at the beginning of the message. The
        /// header will be 10 bytes for an IPv4 address, 22 bytes for an IPv6
        /// address, and 7 bytes plus the length of the domain for a domain address.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn send_to<A>(&self, buf: &[u8], addr: &A) -> io::Result<usize>
        where
            A: ToTargetAddr,
        {
            let addr = addr.to_target_addr()?;

            let mut header = [0; MAX_ADDR_LEN + 3];
            // first two bytes are reserved at 0
            // third byte is the fragment id at 0
            let len = write_addr(&mut header[3..], &addr)?;

            // TODO: Use write_vectored?
            self.socket.writev([&header[..len + 3], buf])
        }

        /// Like `UdpSocket::recv_from`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, TargetAddr)> {
            let mut header = [0; MAX_ADDR_LEN + 3];
            // TODO: Use read_vectored?
            let len = self.socket.readv([&mut header, buf])?;

            let overflow = len.saturating_sub(header.len());

            let header_len = cmp::min(header.len(), len);
            let mut header = &mut &header[..header_len];

            {
                let bytes = header.read_u16::<BigEndian>()?;
                if bytes != 0 {
                    return Err(Error::InvalidReservedBytes { bytes }.into());
                }
            }
            {
                let fid = header.read_u8()?;
                if fid != 0 {
                    return Err(Error::InvalidFragmentID { fid }.into_io());
                }
            }
            let addr = read_addr(&mut header)?;

            unsafe {
                ptr::copy(buf.as_ptr(), buf.as_mut_ptr().add(header.len()), overflow);
            }
            buf[..header.len()].copy_from_slice(header);

            Ok((header.len() + overflow, addr))
        }

        /// Returns the address of the proxy-side UDP socket through which all
        /// messages will be routed.
        #[must_use]
        pub const fn proxy_addr(&self) -> &TargetAddr {
            &self.stream.proxy_addr
        }

        /// Returns a shared reference to the inner socket.
        #[must_use]
        pub const fn get_ref(&self) -> &UdpSocket {
            &self.socket
        }

        /// Returns a mutable reference to the inner socket.
        pub fn get_mut(&mut self) -> &mut UdpSocket {
            &mut self.socket
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod test {
    #[cfg(feature = "bind")]
    use super::bind::*;
    #[cfg(feature = "client")]
    use super::client::*;
    #[cfg(feature = "udp")]
    use super::udp::*;

    use super::*;
    use crate::unwrap_io_to_socks2_error;
    use core::time::Duration;
    use std::{
        io::{Read, Write},
        net::{TcpStream, ToSocketAddrs, UdpSocket},
    };

    const SOCKS_PROXY_NO_AUTH_ONLY: &str = "127.0.0.1:1084";
    const SOCKS_PROXY_PASSWD_ONLY: &str = "127.0.0.1:1085";

    #[test]
    #[cfg(feature = "client")]
    fn google_no_auth() {
        let addr = "google.com:80".to_socket_addrs().unwrap().next().unwrap();
        let socket = Socks5Stream::connect(
            SOCKS_PROXY_NO_AUTH_ONLY,
            &addr,
            Some(Duration::from_secs(10)),
        )
        .unwrap();
        google(socket);
    }

    #[test]
    #[cfg(feature = "client")]
    fn google_with_password() {
        let addr = "google.com:80".to_socket_addrs().unwrap().next().unwrap();
        let socket = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            "testuser",
            "testpass",
            None,
        )
        .unwrap();
        google(socket);
    }

    #[cfg(feature = "client")]
    fn google(mut socket: Socks5Stream) {
        socket.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
        let mut result = vec![];
        socket.read_to_end(&mut result).unwrap();

        println!("{}", String::from_utf8_lossy(&result));
        assert!(result.starts_with(b"HTTP/1.0"));
        assert!(result.ends_with(b"</HTML>\r\n") || result.ends_with(b"</html>"));
    }

    #[test]
    #[cfg(feature = "client")]
    fn google_dns() {
        let mut socket =
            Socks5Stream::connect(SOCKS_PROXY_NO_AUTH_ONLY, &"google.com:80", None).unwrap();

        socket.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
        let mut result = vec![];
        socket.read_to_end(&mut result).unwrap();

        println!("{}", String::from_utf8_lossy(&result));
        assert!(result.starts_with(b"HTTP/1.0"));
        assert!(result.ends_with(b"</HTML>\r\n") || result.ends_with(b"</html>"));
    }

    #[test]
    #[cfg(feature = "bind")]
    fn bind_no_auth() {
        let addr = find_address();
        let listener = Socks5Listener::bind(SOCKS_PROXY_NO_AUTH_ONLY, &addr, None).unwrap();
        bind(listener);
    }

    #[test]
    #[cfg(feature = "bind")]
    fn bind_with_password_supported_but_no_auth_used() {
        let addr = find_address();
        let listener = Socks5Listener::bind_with_password(
            SOCKS_PROXY_NO_AUTH_ONLY,
            &addr,
            "unused_and_invalid_username",
            "unused_and_invalid_password",
            None,
        )
        .unwrap();
        bind(listener);
    }

    #[test]
    #[cfg(feature = "bind")]
    fn bind_with_password() {
        let addr = find_address();
        let listener = Socks5Listener::bind_with_password(
            "127.0.0.1:1085",
            &addr,
            "testuser",
            "testpass",
            None,
        )
        .unwrap();
        bind(listener);
    }

    #[cfg(feature = "bind")]
    fn bind(listener: Socks5Listener) {
        let addr = listener.proxy_addr().clone();
        let mut end = TcpStream::connect(addr).unwrap();
        let mut conn = listener.accept().unwrap();
        conn.write_all(b"hello world").unwrap();
        drop(conn);
        let mut result = vec![];
        end.read_to_end(&mut result).unwrap();
        assert_eq!(result, b"hello world");
    }

    // First figure out our local address that we'll be connecting from
    #[cfg(feature = "client")]
    fn find_address() -> TargetAddr {
        let socket =
            Socks5Stream::connect(SOCKS_PROXY_NO_AUTH_ONLY, &"google.com:80", None).unwrap();
        socket.proxy_addr().to_owned()
    }

    #[test]
    #[cfg(feature = "udp")]
    fn associate_no_auth() {
        let socks =
            Socks5Datagram::bind(SOCKS_PROXY_NO_AUTH_ONLY, "127.0.0.1:15410", None).unwrap();
        associate(&socks, "127.0.0.1:15411");
    }

    #[test]
    #[cfg(feature = "udp")]
    fn associate_with_password() {
        let socks = Socks5Datagram::bind_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            "127.0.0.1:15414",
            "testuser",
            "testpass",
            None,
        )
        .unwrap();
        associate(&socks, "127.0.0.1:15415");
    }

    #[cfg(feature = "udp")]
    fn associate(socks: &Socks5Datagram, socket_addr: &str) {
        let socket = UdpSocket::bind(socket_addr).unwrap();

        socks.send_to(b"hello world!", &socket_addr).unwrap();
        let mut buf = [0; 13];
        let (len, addr) = socket.recv_from(&mut buf).unwrap();
        assert_eq!(len, 12);
        assert_eq!(&buf[..12], b"hello world!");

        socket.send_to(b"hello world!", addr).unwrap();

        let len = socks.recv_from(&mut buf).unwrap().0;
        assert_eq!(len, 12);
        assert_eq!(&buf[..12], b"hello world!");
    }

    #[test]
    #[cfg(feature = "udp")]
    #[allow(clippy::cast_possible_truncation)]
    fn associate_long() {
        let socks =
            Socks5Datagram::bind(SOCKS_PROXY_NO_AUTH_ONLY, "127.0.0.1:15412", None).unwrap();
        let socket_addr = "127.0.0.1:15413";
        let socket = UdpSocket::bind(socket_addr).unwrap();

        let mut msg = vec![];
        for i in 0..(MAX_ADDR_LEN + 100) {
            msg.push(i as u8);
        }

        socks.send_to(&msg, &socket_addr).unwrap();
        let mut buf = vec![0; msg.len() + 1];
        let (len, addr) = socket.recv_from(&mut buf).unwrap();
        assert_eq!(len, msg.len());
        assert_eq!(msg, &buf[..msg.len()]);

        socket.send_to(&msg, addr).unwrap();

        let mut buf = vec![0; msg.len() + 1];
        let len = socks.recv_from(&mut buf).unwrap().0;
        assert_eq!(len, msg.len());
        assert_eq!(msg, &buf[..msg.len()]);
    }

    #[test]
    #[cfg(feature = "client")]
    fn incorrect_password() {
        let addr = "google.com:80".to_socket_addrs().unwrap().next().unwrap();
        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            "testuser",
            "invalid",
            None,
        )
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(err.to_string(), "password authentication failed");
    }

    #[test]
    #[cfg(feature = "client")]
    fn auth_method_not_supported() {
        let addr = "google.com:80".to_socket_addrs().unwrap().next().unwrap();
        let err = Socks5Stream::connect(SOCKS_PROXY_PASSWD_ONLY, &addr, None).unwrap_err();

        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::NoAuthMethods { method: 99 })
        );
    }

    #[test]
    #[cfg(feature = "client")]
    fn username_and_password_length() {
        let addr = "google.com:80".to_socket_addrs().unwrap().next().unwrap();

        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            &string_of_size(1),
            &string_of_size(1),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::FailedPasswordAuth {})
        );

        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            &string_of_size(255),
            &string_of_size(255),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::FailedPasswordAuth {})
        );

        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            &string_of_size(0),
            &string_of_size(255),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::InvalidUsername {
                username: "1".to_string(),
                length: 1
            })
        );

        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            &string_of_size(256),
            &string_of_size(255),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::InvalidUsername {
                username: "1".to_string(),
                length: 1
            })
        );

        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            &string_of_size(255),
            &string_of_size(0),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::InvalidPassword {
                password: (),
                length: 1
            })
        );

        let err = Socks5Stream::connect_with_password(
            SOCKS_PROXY_PASSWD_ONLY,
            &addr,
            &string_of_size(255),
            &string_of_size(256),
            None,
        )
        .unwrap_err();
        assert_eq!(
            unwrap_io_to_socks2_error(&err),
            Some(&Error::InvalidPassword {
                password: (),
                length: 1
            })
        );
    }

    #[cfg(feature = "client")]
    fn string_of_size(size: usize) -> String {
        (0..size).map(|_| 'x').collect()
    }
}
