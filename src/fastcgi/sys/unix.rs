use libc as c;
use std::io::{self, Read, Write};
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::unix::io::RawFd;

use super::cvt;

const LISTENSOCK_FILENO: c::c_int = 0;

pub struct Transport {
    inner: c::c_int,
}

impl Transport {
    pub fn new() -> Self {
        Self::from_raw_fd(LISTENSOCK_FILENO)
    }

    pub fn from_raw_fd(raw_fd: RawFd) -> Self {
        Transport { inner: raw_fd }
    }

    pub fn accept(&mut self) -> io::Result<Socket> {

        let sock = unsafe {
            cvt(c::accept(self.inner, 0 as *mut _, 0 as *mut _))?
        };

        Ok(Socket { inner: sock })
    }
}

pub struct Socket {
    inner: c::c_int,
}

impl Socket {
    pub fn peer(&self) -> io::Result<String> {
        unsafe {
            let mut ss = mem::zeroed::<c::sockaddr_storage>();
            let mut len = mem::size_of::<c::sockaddr_storage>() as c::socklen_t;

            cvt(c::getpeername(
                self.inner,
                &mut ss as *mut _ as *mut c::sockaddr,
                &mut len
            ))?;

            match ss.ss_family as c::c_int {
                c::AF_INET => {
                    let sin = *(&ss as *const _ as *const c::sockaddr_in);
                    let ip = mem::transmute::<c::in_addr, [u8; 4]>(sin.sin_addr);
                    Ok(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]).to_string())
                },
                c::AF_INET6 => {
                    let sin = *(&ss as *const _ as *const c::sockaddr_in6);
                    let ip = mem::transmute::<c::in6_addr, [u16; 8]>(sin.sin6_addr);
                    Ok(Ipv6Addr::new(
                            ip[0], ip[1], ip[2], ip[3],
                            ip[4], ip[5], ip[6], ip[7]
                        ).to_string()
                    )
                },
                c::AF_UNIX => Ok(String::new()),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Unsupported FastCGI socket"
                )),
            }
        }
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        unsafe { c::shutdown(self.inner, c::SHUT_WR); }
        let mut buf = Vec::new();
        self.read_to_end(&mut buf).ok();
        unsafe { c::close(self.inner); }
    }
}

impl<'a> Read for &'a Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let res = unsafe {
            cvt(c::read(
                self.inner,
                buf.as_mut_ptr() as *mut c::c_void,
                buf.len() as c::size_t
            ))?
        };

        Ok(res as usize)
    }
}

impl<'a> Write for &'a Socket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = unsafe {
            cvt(c::write(
                self.inner,
                buf.as_ptr() as *const c::c_void,
                buf.len() as c::size_t
            ))?
        };

        Ok(res as usize)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
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
