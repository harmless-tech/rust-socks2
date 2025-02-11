use crate::Error;
use byteorder::{BigEndian, ReadBytesExt};
use std::{
    io::{
        Read, {self},
    },
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
};

const NULL_BYTE: &[u8] = &0_u8.to_be_bytes();

fn read_response(socket: &mut TcpStream) -> io::Result<SocketAddrV4> {
    let mut response = [0u8; 8];
    socket.read_exact(&mut response)?;
    let mut response = &response[..];

    {
        let version = response.read_u8()?;
        if version != 0 {
            return Err(Error::InvalidResponseVersion { version }.into_io());
        }
    }

    match response.read_u8()? {
        90 => {}
        91 => return Err(Error::ConnectionRefused { code: 91 }.into_io()),
        92 => return Err(Error::RejectedRequestID { code: 92 }.into_io()),
        93 => return Err(Error::RejectedRequestID { code: 93 }.into_io()),
        code => return Err(Error::UnknownResponseCode { code }.into_io()),
    }

    let port = response.read_u16::<BigEndian>()?;
    let ip = Ipv4Addr::from(response.read_u32::<BigEndian>()?);

    Ok(SocketAddrV4::new(ip, port))
}

#[cfg(feature = "client")]
pub mod client {
    use crate::{
        tcp_stream_connect,
        v4::{read_response, NULL_BYTE},
        Error, TargetAddr, ToTargetAddr,
    };
    use core::time::Duration;
    use std::{
        io,
        io::{Read, Write},
        net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream, ToSocketAddrs},
    };

    /// A SOCKS4 and SOCKS4A client.
    #[derive(Debug)]
    pub struct Socks4Stream {
        pub(super) socket: TcpStream,
        pub(super) proxy_addr: SocketAddrV4,
    }

    impl Socks4Stream {
        /// Connects to a target server through a SOCKS4 proxy.
        ///
        /// # Notes
        /// If `target` is a `TargetAddr::Domain`, the domain name will be forwarded
        /// to the proxy server using the SOCKS4A protocol extension. If the proxy
        /// server does not support SOCKS4A, consider performing the DNS lookup
        /// locally and passing a `TargetAddr::Ip`.
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
            userid: &str,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            Self::connect_raw(1, proxy, target, userid, connect_timeout)
        }

        pub(super) fn connect_raw<T, U>(
            command: u8,
            proxy: T,
            target: &U,
            userid: &str,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            let mut socket = tcp_stream_connect(proxy, connect_timeout)?;
            let target = target.to_target_addr()?;

            let mut packet = vec![];
            packet.write_all(&4_u8.to_be_bytes())?; // version
            packet.write_all(&command.to_be_bytes())?; // command code
            match target.to_target_addr()? {
                TargetAddr::Ip(addr) => {
                    let addr = match addr {
                        SocketAddr::V4(addr) => addr,
                        SocketAddr::V6(addr) => {
                            return Err(Error::Socks4NoIPv6 { addr }.into_io());
                        }
                    };
                    packet.write_all(&addr.port().to_be_bytes())?;
                    packet.write_all(&addr.ip().octets())?;
                    packet.write_all(userid.as_bytes())?;
                    packet.write_all(NULL_BYTE)?;
                }
                TargetAddr::Domain(ref host, port) => {
                    packet.write_all(&port.to_be_bytes())?;
                    packet.write_all(&Ipv4Addr::new(0, 0, 0, 1).octets())?;
                    packet.write_all(userid.as_bytes())?;
                    packet.write_all(NULL_BYTE)?;
                    packet.write_all(host.as_bytes())?;
                    packet.write_all(NULL_BYTE)?;
                }
            }

            socket.write_all(&packet)?;
            let proxy_addr = read_response(&mut socket)?;

            Ok(Self { socket, proxy_addr })
        }

        /// Returns the proxy-side address of the connection between the proxy and
        /// target server.
        #[must_use]
        pub const fn proxy_addr(&self) -> SocketAddrV4 {
            self.proxy_addr
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

        /// Consumes the `Socks4Stream`, returning the inner `TcpStream`.
        #[must_use]
        pub fn into_inner(self) -> TcpStream {
            self.socket
        }
    }

    impl Read for Socks4Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.socket.read(buf)
        }
    }

    impl Read for &Socks4Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            (&self.socket).read(buf)
        }
    }

    impl Write for Socks4Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.socket.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.socket.flush()
        }
    }

    impl Write for &Socks4Stream {
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
        v4::{client::Socks4Stream, read_response},
        ToTargetAddr,
    };
    use core::time::Duration;
    use std::{
        io,
        net::{SocketAddr, SocketAddrV4, SocketAddrV6, ToSocketAddrs},
    };

    /// A SOCKS4 and SOCKS4A BIND client.
    #[derive(Debug)]
    pub struct Socks4Listener(Socks4Stream);

    impl Socks4Listener {
        /// Initiates a BIND request to the specified proxy.
        ///
        /// The proxy will filter incoming connections based on the value of
        /// `target`.
        ///
        /// # Notes
        /// See `Socks4Stream::connect()`.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn bind<T, U>(
            proxy: T,
            target: &U,
            userid: &str,
            connect_timeout: Option<Duration>,
        ) -> io::Result<Self>
        where
            T: ToSocketAddrs,
            U: ToTargetAddr,
        {
            Socks4Stream::connect_raw(2, proxy, target, userid, connect_timeout).map(Socks4Listener)
        }

        /// The address of the proxy-side TCP listener.
        ///
        /// This should be forwarded to the remote process, which should open a
        /// connection to it.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn proxy_addr(&self) -> io::Result<SocketAddr> {
            if self.0.proxy_addr.ip().octets() == [0, 0, 0, 0] {
                let port = self.0.proxy_addr.port();
                let peer = match self.0.socket.peer_addr()? {
                    SocketAddr::V4(addr) => SocketAddr::V4(SocketAddrV4::new(*addr.ip(), port)),
                    SocketAddr::V6(addr) => {
                        SocketAddr::V6(SocketAddrV6::new(*addr.ip(), port, 0, 0))
                    }
                };
                Ok(peer)
            } else {
                Ok(SocketAddr::V4(self.0.proxy_addr()))
            }
        }

        /// Waits for the remote process to connect to the proxy server.
        ///
        /// The value of `proxy_addr` should be forwarded to the remote process
        /// before this method is called.
        ///
        /// # Errors
        /// - `io::Error(std::io::ErrorKind::*, socks2::Error::*?)`
        pub fn accept(mut self) -> io::Result<Socks4Stream> {
            self.0.proxy_addr = read_response(&mut self.0.socket)?;
            Ok(self.0)
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

    use core::time::Duration;
    use std::{
        io::{Read, Write},
        net::{SocketAddr, SocketAddrV4, TcpStream, ToSocketAddrs},
    };

    const PROXY_ADDR: &str = "127.0.0.1:1084";

    fn google_ip() -> SocketAddrV4 {
        "google.com:80"
            .to_socket_addrs()
            .unwrap()
            .find_map(|a| match a {
                SocketAddr::V4(a) => Some(a),
                SocketAddr::V6(_) => None,
            })
            .unwrap()
    }

    #[test]
    #[cfg(feature = "client")]
    fn google() {
        let mut socket = Socks4Stream::connect(PROXY_ADDR, &google_ip(), "", None).unwrap();

        socket.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
        let mut result = vec![];
        socket.read_to_end(&mut result).unwrap();

        println!("{}", String::from_utf8_lossy(&result));
        assert!(result.starts_with(b"HTTP/1.0"));
        assert!(result.ends_with(b"</HTML>\r\n") || result.ends_with(b"</html>"));
    }

    #[test]
    #[ignore] // dante doesn't support SOCKS4A
    #[cfg(feature = "client")]
    fn google_dns() {
        let mut socket = Socks4Stream::connect(
            PROXY_ADDR,
            &"google.com:80",
            "",
            Some(Duration::from_secs(10)),
        )
        .unwrap();

        socket.write_all(b"GET / HTTP/1.0\r\n\r\n").unwrap();
        let mut result = vec![];
        socket.read_to_end(&mut result).unwrap();

        println!("{}", String::from_utf8_lossy(&result));
        assert!(result.starts_with(b"HTTP/1.0"));
        assert!(result.ends_with(b"</HTML>\r\n") || result.ends_with(b"</html>"));
    }

    #[test]
    #[cfg(feature = "bind")]
    fn bind() {
        // First figure out our local address that we'll be connecting from
        let socket = Socks4Stream::connect(PROXY_ADDR, &google_ip(), "", None).unwrap();
        let addr = socket.proxy_addr();

        let listener = Socks4Listener::bind(PROXY_ADDR, &addr, "", None).unwrap();
        let addr = listener.proxy_addr().unwrap();
        let mut end = TcpStream::connect(addr).unwrap();
        let mut conn = listener.accept().unwrap();
        conn.write_all(b"hello world").unwrap();
        drop(conn);
        let mut result = vec![];
        end.read_to_end(&mut result).unwrap();
        assert_eq!(result, b"hello world");
    }
}
