#![allow(dead_code)]

use std::cmp;
use std::borrow::Borrow;
use std::io;
use std::io::prelude::*;

use buf_redux::BufReader;
use buf_redux::policy::MinBuffered;
use twoway;

use self::State::*;

pub const MIN_BUF_SIZE: usize = 1024;

#[derive(Debug, PartialEq, Eq)]
enum State {
    Searching,
    BoundaryRead,
    AtEnd
}

/// A struct implementing `Read` and `BufRead` that will yield bytes until it sees a given sequence.
#[derive(Debug)]
pub struct BoundaryReader<R> {
    source: BufReader<R, MinBuffered>,
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

        let source = BufReader::new(reader).set_policy(MinBuffered(MIN_BUF_SIZE));

        BoundaryReader {
            source,
            boundary,
            search_idx: 0,
            state: Searching,
        }
    }

    fn read_to_boundary(&mut self) -> io::Result<&[u8]> {
        // // Make sure there's enough bytes in the buffer to positively identify the boundary.
        // let min_len = self.search_idx + (self.boundary.len() * 2);

        // let buf = fill_buf_min(&mut self.source, min_len)?;

        // if buf.is_empty() {
        //     println!("fill_buf_min returned zero-sized buf");
        // }
        let buf = self.source.fill_buf()?;

        if self.state == BoundaryRead || self.state == AtEnd {
            return Ok(&buf[..self.search_idx])
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

        if self.search_idx >= 2 && !buf[self.search_idx..].starts_with(b"\r\n") {
            let two_bytes_before = &buf[self.search_idx - 2 .. self.search_idx];

            if two_bytes_before == *b"\r\n" {
                self.search_idx -= 2;
            }
        }

        let ret_buf = &buf[..self.search_idx];

        Ok(ret_buf)
    }

    pub fn set_min_buf_size(&mut self, min_buf_size: usize) {
        // ensure the minimum buf size is at least enough to find a boundary with some extra
        let min_buf_size = cmp::max(self.boundary.len() * 2, min_buf_size);

        self.source.policy_mut().0 = min_buf_size;
    }

    #[doc(hidden)]
    pub fn consume_boundary(&mut self) -> io::Result<bool> {
        if self.state == AtEnd {
            return Ok(true);
        }

        while self.state == Searching {
            let buf_len = self.read_to_boundary()?.len();

            if buf_len == 0 && self.state == Searching {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof,
                                          "unexpected end of request body"));
            }

            self.consume(buf_len);
        }

        let consume_amt = {
            let buf = self.source.fill_buf()?;

            // if the boundary is found we should have at least this much in-buffer
            let mut consume_amt = self.search_idx + self.boundary.len();

            // we don't care about data before the cursor
            let bnd_segment = &buf[self.search_idx..];

            if bnd_segment.starts_with(b"\r\n") {
                // preceding CRLF needs to be consumed as well
                consume_amt += 2;

                // assert that we've found the boundary after the CRLF
                debug_assert_eq!(*self.boundary, bnd_segment[2 .. self.boundary.len() + 2]);
            } else {
                // assert that we've found the boundary
                debug_assert_eq!(*self.boundary, bnd_segment[..self.boundary.len()]);
            }

            // include the trailing CRLF or --
            consume_amt += 2;

            if buf.len() < consume_amt {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof,
                                          "not enough bytes to verify boundary"));
            }

            // we have enough bytes to verify
            self.state = Searching;

            let last_two = &buf[consume_amt - 2 .. consume_amt];

            match last_two {
                b"\r\n" => self.state = Searching,
                b"--" => self.state = AtEnd,
                _ => return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected bytes following multipart boundary: {:X} {:X}",
                            last_two[0], last_two[1])
                )),
            }

            consume_amt
        };

        self.source.consume(consume_amt);
        self.search_idx = 0;

        Ok(self.state != AtEnd)
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
