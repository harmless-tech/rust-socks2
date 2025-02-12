use core::mem::size_of;
use std::io;

pub trait BytesExt: io::Read {
    #[inline]
    fn read_be_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0_u8; size_of::<u8>()];
        self.read_exact(&mut buf)?;
        Ok(u8::from_be_bytes(buf))
    }

    #[inline]
    fn read_be_u16(&mut self) -> io::Result<u16> {
        let mut buf = [0_u8; size_of::<u16>()];
        self.read_exact(&mut buf)?;
        Ok(u16::from_be_bytes(buf))
    }

    #[inline]
    fn read_be_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0_u8; size_of::<u32>()];
        self.read_exact(&mut buf)?;
        Ok(u32::from_be_bytes(buf))
    }
}

impl<T: io::Read> BytesExt for T {}
