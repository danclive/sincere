use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream, SocketAddr};

pub struct Transport<'a> {
    inner: &'a TcpListener,
}

impl<'a> Transport<'a> {
    pub fn from_tcp(listener: &'a TcpListener) -> Self {
        Transport { inner: listener }
    }

    pub fn accept(&mut self) -> io::Result<Socket> {
        let (stream, _) = self.inner.accept()?;
        Ok(Socket { inner: stream })
    }
}

pub struct Socket {
    inner: TcpStream,
}

impl Socket {
    pub fn peer(&self) -> io::Result<String> {
        match try!(self.inner.peer_addr()) {
            SocketAddr::V4(addr) => Ok(addr.ip().to_string()),
            SocketAddr::V6(addr) => Ok(addr.ip().to_string()),
        }
    }
}

impl<'a> Read for &'a Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.inner).read(buf)
    }
}

impl<'a> Write for &'a Socket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.inner).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&self.inner).flush()
    }
}

impl Read for Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&*self).read(buf)
    }
}

impl Write for Socket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&*self).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&*self).flush()
    }
}
