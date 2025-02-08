use std::{io, net::UdpSocket};

pub trait WritevExt {
    fn writev(&self, bufs: [&[u8]; 2]) -> io::Result<usize>;
    fn readv(&self, bufs: [&mut [u8]; 2]) -> io::Result<usize>;
}

const VEC_SIZE: usize = 2;

#[cfg(unix)]
mod imp {
    use std::os::unix::io::AsRawFd;

    use super::{io, UdpSocket, WritevExt, VEC_SIZE};

    // TODO: Make iovecs pointer casts the same???
    impl WritevExt for UdpSocket {
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
    use super::*;
    use std::{os::windows::io::AsRawSocket, ptr};
    use windows_sys::Win32::Networking::WinSock::{WSARecv, WSASend, WSABUF};

    impl WritevExt for UdpSocket {
        fn writev(&self, bufs: [&[u8]; VEC_SIZE]) -> io::Result<usize> {
            // TODO: Check to make sure length is within a u32!

            let mut wsabufs: [WSABUF; VEC_SIZE] = [
                WSABUF {
                    len: bufs[0].len() as _, // TODO: Casts to u32!!!
                    buf: bufs[0].as_ptr().cast_mut(),
                },
                WSABUF {
                    len: bufs[1].len() as _, // TODO: Casts to u32!!!
                    buf: bufs[1].as_ptr().cast_mut(),
                },
            ];
            let mut sent = 0_u32;
            // SAFETY: All params are setup in this function safely.
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
            // TODO: Check to make sure length is within a u32!

            let mut wsabufs: [WSABUF; VEC_SIZE] = [
                WSABUF {
                    len: bufs[0].len() as _, // TODO: Casts to u32!!!
                    buf: bufs[0].as_mut_ptr(),
                },
                WSABUF {
                    len: bufs[1].len() as _, // TODO: Casts to u32!!!
                    buf: bufs[1].as_mut_ptr(),
                },
            ];
            let mut recved: u32 = 0;
            let mut flags: u32 = 0;
            // SAFETY: All params are setup in this function safely.
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
