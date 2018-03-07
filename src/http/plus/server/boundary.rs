#![allow(dead_code)]

use std::cmp;
use std::borrow::Borrow;
use std::io;
use std::io::prelude::*;

use buf_redux::BufReader;
use twoway;

use self::State::*;

#[derive(Debug, PartialEq, Eq)]
enum State {
    Searching,
    BoundaryRead,
    AtEnd
}

/// A struct implementing `Read` and `BufRead` that will yield bytes until it sees a given sequence.
#[derive(Debug)]
pub struct BoundaryReader<R> {
    source: BufReader<R>,
    boundary: Vec<u8>,
    search_idx: usize,
    state: State,
}

impl<R> BoundaryReader<R> where R: Read {
    #[doc(hidden)]
    pub fn from_reader<B: Into<Vec<u8>>>(reader: R, boundary: B) -> BoundaryReader<R> {
        let boundary_temp = boundary.into();

        let mut boundary: Vec<u8> = Vec::with_capacity(boundary_temp.len() + 2);
        boundary.push(b'-');
        boundary.push(b'-');
        boundary.extend(boundary_temp);

        BoundaryReader {
            source: BufReader::new(reader),
            boundary: boundary,
            search_idx: 0,
            state: Searching,
        }
    }

    fn read_to_boundary(&mut self) -> io::Result<&[u8]> {
        // Make sure there's enough bytes in the buffer to positively identify the boundary.
        let min_len = self.search_idx + (self.boundary.len() * 2);

        let buf = fill_buf_min(&mut self.source, min_len)?;

        if buf.is_empty() {
            println!("fill_buf_min returned zero-sized buf");
        }

        if self.state == Searching && self.search_idx < buf.len() {
            let lookahead = &buf[self.search_idx..];

            // Look for the boundary, or if it isn't found, stop near the end.
            match twoway::find_bytes(lookahead, &self.boundary) {
                Some(found_idx) => {
                    self.search_idx += found_idx;
                    self.state = BoundaryRead;
                },
                None => {
                    self.search_idx += lookahead.len().saturating_sub(self.boundary.len() + 2);
                }
            }
        }

        // don't modify search_idx so it always points to the start of the boundary
        let mut buf_len = self.search_idx;

        // back up the cursor to before the boundary's preceding CRLF
        if self.state != Searching && buf_len >= 2 {
            let two_bytes_before = &buf[buf_len - 2 .. buf_len];

            if two_bytes_before == &*b"\r\n" {
                buf_len -= 2;
            }
        }

        let ret_buf = &buf[..buf_len];

        Ok(ret_buf)
    }

    #[doc(hidden)]
    pub fn consume_boundary(&mut self) -> io::Result<bool> {
        if self.state == AtEnd {
            return Ok(true);
        }

        while self.state == Searching {
            let buf_len = self.read_to_boundary()?.len();

            self.consume(buf_len);
        }

        let consume_amt = {
            let min_len = self.boundary.len() + 4;

            let buf = fill_buf_min(&mut self.source, min_len)?;

            if buf.len() < min_len {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof,
                                          "not enough bytes to verify boundary"));
            }

            // we have enough bytes to verify
            self.state = Searching;

            let mut consume_amt = self.search_idx + self.boundary.len();

            let last_two = &buf[consume_amt .. consume_amt + 2];

            match last_two {
                b"\r\n" => consume_amt += 2,
                b"--" => { consume_amt += 2; self.state = AtEnd },
                _ => ()
            }

            consume_amt
        };

        self.source.consume(consume_amt);
        self.search_idx = 0;

        Ok(self.state == AtEnd)
    }
}

#[cfg(feature = "bench")]
impl<'a> BoundaryReader<io::Cursor<&'a [u8]>> {
    fn new_with_bytes(bytes: &'a [u8], boundary: &str) -> Self {
        Self::from_reader(io::Cursor::new(bytes), boundary)
    }

    fn reset(&mut self) {
        // Dump buffer and reset cursor
        self.source.seek(io::SeekFrom::Start(0));
        self.state = Searching;
        self.search_idx = 0;
    }
}

impl<R> Borrow<R> for BoundaryReader<R> {
    fn borrow(&self) -> &R {
        self.source.get_ref()
    }
}

impl<R> Read for BoundaryReader<R> where R: Read {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let read = {
            let mut buf = self.read_to_boundary()?;
            // This shouldn't ever be an error so unwrapping is fine.
            buf.read(out).unwrap()
        };

        self.consume(read);
        Ok(read)
    }
}

impl<R> BufRead for BoundaryReader<R> where R: Read {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.read_to_boundary()
    }

    fn consume(&mut self, amt: usize) {
        let true_amt = cmp::min(amt, self.search_idx);

        self.source.consume(true_amt);
        self.search_idx -= true_amt;
    }
}

fn fill_buf_min<R: Read>(buf: &mut BufReader<R>, min: usize) -> io::Result<&[u8]> {
    let mut attempts = 0;

    while buf.available() < min && attempts < min {
        if buf.read_into_buf()? == 0 { break; };
        attempts += 1;
    }

    Ok(buf.get_buf())
}
