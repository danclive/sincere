use std::sync::{Arc, Mutex};
use std::io::Read;
use std::io::Write;
use std::io::Result as IoResult;
use std::cmp;
use std::net::SocketAddr;
use std::mem;

#[derive(Clone, Debug)]
pub struct Stream {
    reader: Vec<u8>,
    writer: Arc<Mutex<Vec<u8>>>,
    remote_addr: SocketAddr,
}

impl Stream {
    pub fn new(reader: Vec<u8>, writer: Arc<Mutex<Vec<u8>>>, remote_addr: SocketAddr) -> Stream {
        Stream {
            reader: reader,
            writer: writer,
            remote_addr: remote_addr,
        }
    }

    pub fn len(&self) -> usize {
        self.reader.len()
    }

    pub fn get(&self, index: usize) -> Option<&u8> {
        self.reader.get(index)
    }

    pub fn split_off(&mut self, at: usize) {
        self.reader = self.reader.split_off(at);
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr
    }

    pub fn to_vec(&mut self) -> Vec<u8> {
        mem::replace(&mut self.reader, Vec::new())
    }
}

impl Read for Stream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let amt = cmp::min(buf.len(), self.reader.len());
        
        let reader = self.reader.clone();
        let (a, b) = reader.split_at(amt);
        buf[..amt].copy_from_slice(a);
        self.reader = b.to_vec();
        
        Ok(amt)
    }
}

impl Write for Stream {
    #[inline]
    fn write(&mut self, data: &[u8]) -> IoResult<usize> {
        let mut writer = self.writer.lock().unwrap();
        writer.write(data)
    }

    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}
