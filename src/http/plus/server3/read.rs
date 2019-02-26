use std::io::{Read, BufRead, BufReader};
use std::io::Result;

use httparse;

use crate::util::buf_read_ext::BufReadExt;


#[derive(Clone, Debug)]
pub enum Node {
    /// A part in memory
    Part(Part),
    /// A part streamed to a file
    File(FilePart),
    // /// A container of nested multipart parts
    // Multipart((Headers, Vec<Node>)),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    //pub headers: Headers,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilePart {
    /// The headers of the part
    //pub headers: Headers,
    /// A temporary file containing the file content
    //pub path: PathBuf,
    /// Optionally, the size of the file.  This is filled when multiparts are parsed, but is
    /// not necessary when they are generated.
    pub size: Option<usize>,
    // The temporary directory the upload was put into, saved for the Drop trait
    //tempdir: Option<PathBuf>,
}

pub fn read_multipart<R: Read>(stream: &mut R, boundary: &str) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut buf: Vec<u8> = Vec::new();
    let (_, found) = reader.stream_until_token(boundary.as_bytes(), &mut buf)?;
    if !found {
        return Ok(())
    }

    println!("/////////////////////{:?}", found);

    let (lt, ltlt, lt_boundary) = {
        let peeker = reader.fill_buf()?;

        //println!("{:?}", String::from_utf8_lossy(peeker));
        if peeker.len() > 1 && &peeker[..2] == b"\r\n" {
            let mut output = Vec::with_capacity(2 + boundary.len());
            output.push(b'\r');
            output.push(b'\n');
            output.extend(boundary.clone().as_bytes());
            (vec![b'\r', b'\n'], vec![b'\r', b'\n', b'\r', b'\n'], output)
        } else if peeker.len() > 0 && peeker[0]==b'\n' {
            let mut output = Vec::with_capacity(1 + boundary.len());
            output.push(b'\n');
            output.extend(boundary.clone().as_bytes());
            (vec![b'\n'], vec![b'\n', b'\n'], output)
        } else {
            return Ok(())
        }
    };

    loop {
        {
            let peeker = reader.fill_buf()?;
            if peeker.len() >= 2 && &peeker[..2] == b"--" {
                return Ok(());
            }
        }

        let (_, found) = reader.stream_until_token(&lt, &mut buf)?;
        if !found {
            panic!("{:?}", found);
        }

        buf.truncate(0);
        let (_, found) = reader.stream_until_token(&ltlt, &mut buf)?;
        if !found {
            panic!("{:?}", found);
        }

        buf.extend(ltlt.iter().cloned());

        let part_headers = {
            let mut header_memory = [httparse::EMPTY_HEADER; 4];
            match httparse::parse_headers(&buf, &mut header_memory) {
                Ok(httparse::Status::Complete((size, raw_headers))) => {
                    //Headers::from_raw(raw_headers).map_err(|e| From::from(e))
                    let mut vec: Vec<(String, String)> = Vec::with_capacity(size);

                    for h in raw_headers {
                        vec.push((h.name.to_owned(), String::from_utf8_lossy(h.value).to_string()));
                    }

                    vec
                },
                // Ok(httparse::Status::Partial) => Err(Error::PartialHeaders),
                // Err(err) => Err(From::from(err)),
                Ok(httparse::Status::Partial) => panic!("{:?}", ""),
                Err(err) => panic!("{:?}", err)
            }
        };
    }

    return Ok(())
}

