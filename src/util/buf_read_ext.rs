use std::io::{BufRead, ErrorKind, Result, Write};

pub trait BufReadExt: BufRead {
    fn stream_until_token<W: Write>(&mut self, token: &[u8], out: &mut W) -> Result<(usize, bool)> {
        stream_until_token(self, token, out)
    }
}

impl<T: BufRead> BufReadExt for T {}

fn stream_until_token<R: BufRead + ?Sized, W: Write>(
    stream: &mut R,
    token: &[u8],
    out: &mut W,
) -> Result<(usize, bool)> {
    let mut read = 0;
    // Represents the sizes of possible token prefixes found at the end of the last buffer, usually
    // empty. If not empty, the beginning of this buffer is checked for the matching suffixes to
    // to find tokens that straddle two buffers. Entries should be in longest prefix to shortest
    // prefix order.
    let mut prefix_lengths: Vec<usize> = Vec::new();
    let mut found: bool;
    let mut used: usize;

    'stream: loop {
        found = false;
        used = 0;

        // This is not actually meant to repeat, we only need the break functionality of a loop.
        // The reader is encouraged to try their hand at coding this better, noting that buffer must
        // drop out of scope before stream can be used again.
        let mut do_once = true;
        'buffer: while do_once {
            do_once = false;

            // Fill the buffer (without consuming)
            let buffer = match stream.fill_buf() {
                Ok(n) => n,
                Err(ref err) if err.kind() == ErrorKind::Interrupted => continue,
                Err(err) => return Err(err),
            };
            if buffer.len() == 0 {
                break 'stream;
            }

            // If the buffer starts with a token suffix matching a token prefix from the end of the
            // previous buffer, then we have found a token.
            if !prefix_lengths.is_empty() {
                let drain: Vec<usize> = prefix_lengths.drain(..).collect();

                for index in 0..drain.len() {
                    let prefix_len = drain[index];

                    let mut prefix_failed: bool = true;

                    // If the buffer is too small to fit an entire suffix
                    if buffer.len() < token.len() - prefix_len {
                        if buffer[..] == token[prefix_len..prefix_len + buffer.len()] {
                            // that prefix just got bigger and needs to be preserved
                            prefix_lengths.push(prefix_len + buffer.len());
                            prefix_failed = false;
                        }
                    } else {
                        // If we find a complete suffix at the front of the buffer for this
                        // prefix...
                        if buffer[..token.len() - prefix_len] == token[prefix_len..] {
                            found = true;
                            used = token.len() - prefix_len;
                            break 'buffer;
                        }
                    }

                    if prefix_failed {
                        // This prefix length doesn't work.  We should write the bytes...
                        if index == drain.len() - 1 {
                            // ...of this prefix length
                            out.write_all(&token[..prefix_len])?;
                        } else {
                            // ...from this prefix length to the next
                            let next_prefix_len = drain[index + 1];
                            out.write_all(&token[..prefix_len - next_prefix_len])?;
                        }
                    }
                }
            }

            // Get the index index of the first token in the middle of the buffer, if any
            let index = buffer
                .windows(token.len())
                .enumerate()
                .filter(|&(_, t)| t == token)
                .map(|(i, _)| i)
                .next();

            if let Some(index) = index {
                out.write_all(&buffer[..index])?;
                found = true;
                used = index + token.len();
                break 'buffer;
            }

            // Check for token prefixes at the end of the buffer.
            let mut window = token.len() - 1;
            if buffer.len() < window {
                window = buffer.len();
            }
            // Remember the largest prefix for writing later if it didn't match
            // (we don't write it now just in case it turns out to be the token)
            let mut reserve = if !prefix_lengths.is_empty() {
                buffer.len()
            } else {
                0
            };
            for prefix in (1..window + 1)
                .rev()
                .filter(|&w| token[..w] == buffer[buffer.len() - w..])
            {
                if reserve == 0 {
                    reserve = prefix;
                }
                prefix_lengths.push(prefix)
            }

            out.write_all(&buffer[..buffer.len() - reserve])?;
            used = buffer.len();
        }

        stream.consume(used);
        read += used;

        if found || used == 0 {
            break;
        }
    }

    return Ok((if found { read - token.len() } else { read }, found));
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::{BufReader, Cursor};

    #[test]
    fn stream_until_token() {
        let mut buf = Cursor::new(&b"123456"[..]);
        let mut result: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"78", &mut result).unwrap(),
            (6, false)
        );
        assert_eq!(result, b"123456");

        let mut buf = Cursor::new(&b"12345678"[..]);
        let mut result: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"34", &mut result).unwrap(),
            (2, true)
        );
        assert_eq!(result, b"12");

        result.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"78", &mut result).unwrap(),
            (2, true)
        );
        assert_eq!(result, b"56");

        let mut buf = Cursor::new(&b"bananas for nana"[..]);
        let mut result: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"nan", &mut result).unwrap(),
            (2, true)
        );
        assert_eq!(result, b"ba");

        result.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"nan", &mut result).unwrap(),
            (7, true)
        );
        assert_eq!(result, b"as for ");

        result.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"nan", &mut result).unwrap(),
            (1, false)
        );
        assert_eq!(result, b"a");

        result.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"nan", &mut result).unwrap(),
            (0, false)
        );
        assert_eq!(result, b"");
    }

    #[test]
    fn stream_until_token_straddle_test() {
        let cursor = Cursor::new(&b"12345TOKEN345678"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut result: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"TOKEN", &mut result).unwrap(),
            (5, true)
        );
        assert_eq!(result, b"12345");

        result.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"TOKEN", &mut result).unwrap(),
            (6, false)
        );
        assert_eq!(result, b"345678");

        result.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"TOKEN", &mut result).unwrap(),
            (0, false)
        );
        assert_eq!(result, b"");

        //                          <------><------><------>
        let cursor = Cursor::new(&b"12345TOKE23456781TOKEN78"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut result: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"TOKEN", &mut result).unwrap(),
            (17, true)
        );
        assert_eq!(result, b"12345TOKE23456781");
    }

    // This tests against mikedilger/formdata github issue #1
    #[test]
    fn stream_until_token_large_token_test() {
        let cursor = Cursor::new(&b"IAMALARGETOKEN7812345678"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"IAMALARGETOKEN", &mut v).unwrap(),
            (0, true)
        );
        assert_eq!(v, b"");
        assert_eq!(
            buf.stream_until_token(b"IAMALARGETOKEN", &mut v).unwrap(),
            (10, false)
        );
        assert_eq!(v, b"7812345678");

        let cursor = Cursor::new(&b"0IAMALARGERTOKEN12345678"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"IAMALARGERTOKEN", &mut v).unwrap(),
            (1, true)
        );
        assert_eq!(v, b"0");
        v.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"IAMALARGERTOKEN", &mut v).unwrap(),
            (8, false)
        );
        assert_eq!(v, b"12345678");
    }

    // This tests against mikedilger/formdata github issue #11
    #[test]
    fn stream_until_token_double_straddle_test() {
        let cursor = Cursor::new(&b"12345IAMALARGETOKEN4567"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"IAMALARGETOKEN", &mut v).unwrap(),
            (5, true)
        );
        assert_eq!(v, b"12345");
        v.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"IAMALARGETOKEN", &mut v).unwrap(),
            (4, false)
        );
        assert_eq!(v, b"4567");
    }

    // This tests against mikedilger/formdata github issue #12
    #[test]
    fn stream_until_token_multiple_prefix_test() {
        let cursor = Cursor::new(&b"12barbarian4567"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"barbarian", &mut v).unwrap(),
            (2, true)
        );
        assert_eq!(v, b"12");

        let cursor = Cursor::new(&b"12barbarbarian7812"[..]);
        let mut buf = BufReader::with_capacity(8, cursor);
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"barbarian", &mut v).unwrap(),
            (5, true)
        );
        assert_eq!(v, b"12bar");
    }

    #[test]
    fn stream_until_token_complex_test() {
        //                                             <-TOKEN->
        //                          <--><--><--><--><--><--><--><-->
        let cursor = Cursor::new(&b"A SANTA BARBARA BARBARBARIANEND"[..]);
        let mut buf = BufReader::with_capacity(4, cursor);
        let mut v: Vec<u8> = Vec::new();
        assert_eq!(
            buf.stream_until_token(b"BARBARIAN", &mut v).unwrap(),
            (19, true)
        );
        assert_eq!(v, b"A SANTA BARBARA BAR");

        /*            prefix lens:   out:
        "A SA"        []             "A SA"
        "NTA "        []             "NTA "
        "BARB"        [4, 1]         ""
        "ARA "        []             "BARB"  "ARA "
        "BARB"        [4, 1]         ""
        "ARBA"        [5, 2]         "BAR"
        "RIAN"
         */

        v.truncate(0);
        assert_eq!(
            buf.stream_until_token(b"BARBARIAN", &mut v).unwrap(),
            (3, false)
        );
        assert_eq!(v, b"END");
    }
}
