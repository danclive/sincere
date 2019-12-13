use std::fmt;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use mime_guess;

use crate::error::Result;
use crate::http::plus::random_alphanumeric;

const BOUNDARY_LEN: usize = 32;

fn gen_boundary() -> String {
    random_alphanumeric(BOUNDARY_LEN)
}

use crate::http::mime::{self, Mime};

/// Create a structure to process the multipart/form-data data format for
/// the client to initiate the request.
///
/// # Examples
///
/// ```
/// use sincere::http::plus::client::Multipart;
///
/// let mut multipart = Multipart::new();
///
/// multipart.add_text("hello", "world");
///
/// let (boundary, data) = multipart.convert().unwrap();
/// ```
///

#[derive(Debug, Default)]
pub struct Multipart<'a> {
    fields: Vec<Field<'a>>,
}

impl<'a> Multipart<'a> {
    /// Returns the empty `Multipart` set.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::http::plus::client;
    ///
    /// let mut multipart = client::Multipart::new();
    /// ```
    #[inline]
    pub fn new() -> Multipart<'a> {
        Multipart { fields: Vec::new() }
    }

    /// Add text 'key-value' into `Multipart`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::http::plus::client;
    ///
    /// let mut multipart = client::Multipart::new();
    ///
    /// multipart.add_text("hello", "world");
    /// ```
    #[inline]
    pub fn add_text<V>(&mut self, name: V, value: V) -> &mut Self
    where
        V: Into<String>,
    {
        let filed = Field {
            name: name.into(),
            data: Data::Text(value.into()),
        };

        self.fields.push(filed);

        self
    }

    /// Add file into `Multipart`.
    ///
    /// # Examples
    ///
    /// ```no_test
    /// use sincere::http::plus::client;
    ///
    /// let mut multipart = client::Multipart::new();
    ///
    /// multipart.add_file("hello.rs", "/aaa/bbb");
    /// ```
    #[inline]
    pub fn add_file<V, P>(&mut self, name: V, path: P) -> &mut Self
    where
        V: Into<String>,
        P: Into<PathBuf>,
    {
        let filed = Field {
            name: name.into(),
            data: Data::File(path.into()),
        };

        self.fields.push(filed);

        self
    }

    /// Add reader stream into `Multipart`.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::http::plus::client;
    ///
    /// let temp = r#"{"hello": "world"}"#.as_bytes();
    /// let reader = ::std::io::Cursor::new(temp);
    ///
    /// let mut multipart = client::Multipart::new();
    ///
    /// multipart.add_stream("ddd", reader, Some("hello.rs"), Some(sincere::http::mime::APPLICATION_JSON));
    /// ```
    #[inline]
    pub fn add_stream<V, R>(
        &mut self,
        name: V,
        stream: R,
        filename: Option<V>,
        mime: Option<Mime>,
    ) -> &mut Self
    where
        R: Read + 'a,
        V: Into<String>,
    {
        let filed = Field {
            name: name.into(),
            data: Data::Stream(Stream {
                content_type: mime.unwrap_or(mime::APPLICATION_OCTET_STREAM),
                filename: filename.map(|f| f.into()),
                stream: Box::new(stream),
            }),
        };

        self.fields.push(filed);

        self
    }

    /// Convert `Multipart` to client boundary and body.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::http::plus::client;
    ///
    /// let mut multipart = client::Multipart::new();
    ///
    /// multipart.add_text("hello", "world");
    ///
    /// let (boundary, data) = multipart.convert().unwrap();
    /// ```
    pub fn convert(&mut self) -> Result<(String, Vec<u8>)> {
        let mut boundary = format!("\r\n--{}", gen_boundary());

        let mut buf: Vec<u8> = Vec::new();

        for field in self.fields.drain(..) {
            match field.data {
                Data::Text(value) => {
                    write!(
                        buf,
                        "{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}",
                        boundary, field.name, value
                    )?;
                }
                Data::File(path) => {
                    let (content_type, filename) = mime_filename(&path);
                    let mut file = File::open(&path)?;

                    write!(
                        buf,
                        "{}\r\nContent-Disposition: form-data; name=\"{}\"",
                        boundary, field.name
                    )?;

                    if let Some(filename) = filename {
                        write!(buf, "; filename=\"{}\"", filename)?;
                    }

                    write!(buf, "\r\nContent-Type: {}\r\n\r\n", content_type)?;

                    let mut temp: Vec<u8> = Vec::new();

                    file.read_to_end(&mut temp)?;

                    buf.extend(temp);
                }
                Data::Stream(mut stream) => {
                    write!(
                        buf,
                        "{}\r\nContent-Disposition: form-data; name=\"{}\"",
                        boundary, field.name
                    )?;

                    if let Some(filename) = stream.filename {
                        write!(buf, "; filename=\"{}\"", filename)?;
                    }

                    write!(buf, "\r\nContent-Type: {}\r\n\r\n", stream.content_type)?;

                    let mut temp: Vec<u8> = Vec::new();

                    stream.stream.read_to_end(&mut temp)?;

                    buf.extend(temp);
                }
            }
        }

        boundary.push_str("--");

        buf.extend(boundary.as_bytes());

        Ok((boundary[4..boundary.len() - 2].to_string(), buf))
    }
}

#[derive(Debug)]
struct Field<'a> {
    name: String,
    data: Data<'a>,
}

enum Data<'a> {
    Text(String),
    File(PathBuf),
    Stream(Stream<'a>),
}

impl<'a> fmt::Debug for Data<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Data::Text(ref value) => write!(f, "Data::Text({:?})", value),
            Data::File(ref path) => write!(f, "Data::File({:?})", path),
            Data::Stream(_) => f.write_str("Data::Stream(Box<Read>)"),
        }
    }
}

struct Stream<'a> {
    filename: Option<String>,
    content_type: Mime,
    stream: Box<dyn Read + 'a>,
}

fn mime_filename(path: &PathBuf) -> (Mime, Option<&str>) {
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    let filename = opt_filename(path);
    (content_type, filename)
}

fn opt_filename(path: &PathBuf) -> Option<&str> {
    path.file_name().and_then(|filename| filename.to_str())
}
