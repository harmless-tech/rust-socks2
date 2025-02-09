use std::{io, net::UdpSocket};

const VEC_SIZE: usize = 2;

pub trait IOVecExt {
    fn writev(&self, bufs: [&[u8]; 2]) -> io::Result<usize>;
    fn readv(&self, bufs: [&mut [u8]; 2]) -> io::Result<usize>;
}

#[cfg(unix)]
mod imp {
    use super::{io, IOVecExt, UdpSocket, VEC_SIZE};
    use std::os::unix::io::AsRawFd;

    impl IOVecExt for UdpSocket {
        fn writev(&self, bufs: [&[u8]; VEC_SIZE]) -> io::Result<usize> {
            let iovecs: [libc::iovec; VEC_SIZE] = [
                libc::iovec {
                    iov_base: bufs[0].as_ptr().cast_mut().cast(),
                    iov_len: bufs[0].len(),
                },
                libc::iovec {
                    iov_base: bufs[1].as_ptr().cast_mut().cast(),
                    iov_len: bufs[1].len(),
                },
            ];

            // SAFETY: All params are setup in this function safely.
            #[allow(clippy::cast_possible_truncation)] // SAFETY: Length is always VEC_SIZE.
            #[allow(clippy::cast_possible_wrap)]
            let r = unsafe { libc::writev(self.as_raw_fd(), iovecs.as_ptr(), VEC_SIZE as _) };

            if r < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(r.unsigned_abs())
            }
        }

        fn readv(&self, bufs: [&mut [u8]; VEC_SIZE]) -> io::Result<usize> {
            let mut iovecs: [libc::iovec; VEC_SIZE] = [
                libc::iovec {
                    iov_base: bufs[0].as_mut_ptr().cast(),
                    iov_len: bufs[0].len(),
                },
                libc::iovec {
                    iov_base: bufs[1].as_mut_ptr().cast(),
                    iov_len: bufs[1].len(),
                },
            ];

            // SAFETY: All params are setup in this function safely.
            #[allow(clippy::cast_possible_truncation)] // SAFETY: Length is always VEC_SIZE.
            #[allow(clippy::cast_possible_wrap)]
            let r = unsafe { libc::readv(self.as_raw_fd(), iovecs.as_mut_ptr(), VEC_SIZE as _) };

            if r < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(r.unsigned_abs())
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::{io, IOVecExt, UdpSocket, VEC_SIZE};
    use crate::Error;
    use std::{os::windows::io::AsRawSocket, ptr};
    use windows_sys::Win32::Networking::WinSock::{WSARecv, WSASend, WSABUF};

    impl IOVecExt for UdpSocket {
        fn writev(&self, bufs: [&[u8]; VEC_SIZE]) -> io::Result<usize> {
            let bufs_lens: [u32; VEC_SIZE] = [
                bufs[0].len().try_into().map_err(|_| {
                    Error::WinUDP4GiBLimit {
                        size: bufs[0].len(),
                    }
                    .into()
                })?,
                bufs[1].len().try_into().map_err(|_| {
                    Error::WinUDP4GiBLimit {
                        size: bufs[1].len(),
                    }
                    .into()
                })?,
            ];

            let mut wsabufs: [WSABUF; VEC_SIZE] = [
                WSABUF {
                    len: bufs_lens[0],
                    buf: bufs[0].as_ptr().cast_mut(),
                },
                WSABUF {
                    len: bufs_lens[1],
                    buf: bufs[1].as_ptr().cast_mut(),
                },
            ];

            let mut sent = 0_u32;
            // SAFETY: All params are setup in this function safely.
            // SAFETY: Length is always VEC_SIZE.
            // SAFETY: On 32 bit systems self.as_raw_socket() returns a u32.
            //         (https://doc.rust-lang.org/src/std/os/windows/raw.rs.html#16)
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_possible_wrap)]
            let r = unsafe {
                WSASend(
                    self.as_raw_socket() as _,
                    wsabufs.as_mut_ptr(),
                    VEC_SIZE as _,
                    &mut sent,
                    0,
                    ptr::null_mut(),
                    None,
                )
            };

            if r == 0 {
                Ok(sent as usize)
            } else {
                Err(io::Error::last_os_error())
            }
        }

        fn readv(&self, bufs: [&mut [u8]; VEC_SIZE]) -> io::Result<usize> {
            let bufs_lens: [u32; VEC_SIZE] = [
                bufs[0].len().try_into().map_err(|_| {
                    Error::WinUDP4GiBLimit {
                        size: bufs[0].len(),
                    }
                    .into()
                })?,
                bufs[1].len().try_into().map_err(|_e| {
                    Error::WinUDP4GiBLimit {
                        size: bufs[1].len(),
                    }
                    .into()
                })?,
            ];

            let mut wsabufs: [WSABUF; VEC_SIZE] = [
                WSABUF {
                    len: bufs_lens[0],
                    buf: bufs[0].as_mut_ptr(),
                },
                WSABUF {
                    len: bufs_lens[1],
                    buf: bufs[1].as_mut_ptr(),
                },
            ];

            let mut recved: u32 = 0;
            let mut flags: u32 = 0;
            // SAFETY: All params are setup in this function safely.
            // SAFETY: Length is always VEC_SIZE.
            // SAFETY: On 32 bit systems self.as_raw_socket() returns a u32.
            //         (https://doc.rust-lang.org/src/std/os/windows/raw.rs.html#16)
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_possible_wrap)]
            let r = unsafe {
                WSARecv(
                    self.as_raw_socket() as _,
                    wsabufs.as_mut_ptr(),
                    VEC_SIZE as _,
                    &mut recved,
                    &mut flags,
                    ptr::null_mut(),
                    None,
                )
            };

            if r == 0 {
                Ok(recved as usize)
            } else {
                Err(io::Error::last_os_error())
            }
        }
    }
}
