use std::path::PathBuf;
use std::fs;
use std::io::{self, Read, Write};

use hyper::mime;

use http::request::Request;
use error::Result;
use http::plus::random_alphanumeric;

use self::multipart::Multipart;

mod multipart;
mod boundary;
mod field;
mod save;

impl Request {
    pub(crate) fn parse_formdata(&mut self) -> Option<FormData> {
        let content_type = match self.content_type() {
            Some(c) => c.to_owned(),
            None => return None
        };

        if content_type.type_() == mime::MULTIPART && content_type.subtype() == mime::FORM_DATA {
            let boundary = if let Some(boundary) = content_type.get_param(mime::BOUNDARY) {
                boundary.as_str()
            } else {
                return None
            };

            let reader = io::Cursor::new(self.body());

            return Some(FormData::with_body(reader, boundary));
        }

        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormData {
    pub fields: Vec<(String, String)>,
    pub files: Vec<FilePart>
}

impl FormData {
    pub fn has_file(&self) -> bool {
        if self.files.len() > 0 {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FilePart {
    pub name: String,
    pub filename: Option<String>,
    pub content_type: mime::Mime,
    pub data: Vec<u8>,
}

impl FilePart {
    pub fn save_file<P: Into<PathBuf>>(&mut self, path: P) -> Result<PathBuf> {
        let mut path_buf = path.into();

        // Temp Path ??
        if let Some(ref filename) = self.filename {
            path_buf.push(filename);
        } else {
            let filename = random_alphanumeric(16);
            path_buf.push(filename);
        }

        let path_buf2 = path_buf.clone();

        let path = path_buf.as_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut open_option = fs::OpenOptions::new();

        open_option.write(true).create(true);

        let mut file = open_option.open(path)?;

        file.write_all(&self.data)?;
        file.flush()?;

        Ok(path_buf2)
    }
}

impl FormData {
    pub fn new() -> FormData {
        FormData {
            fields: Vec::new(),
            files: Vec::new()
        }
    }

    pub fn with_body<R: Read, B: Into<String>>(body: R, boundary: B) -> FormData {
        let mut multipart = Multipart::with_body(body, boundary);

        let mut form_data = FormData::new();

        while let Ok(Some(mut entry)) = multipart.read_entry() {
            if entry.is_text() {
                let mut save_build = entry.data.save();
                let mut buf: Vec<u8> = Vec::new();
                save_build.write_to(&mut buf);

                form_data.fields.push((entry.headers.name.to_string(), String::from_utf8_lossy(&buf).into_owned()));
            } else {
                let mut save_build = entry.data.save();
                let mut buf: Vec<u8> = Vec::new();
                save_build.write_to(&mut buf);

                let file_part = FilePart {
                    name: entry.headers.name.to_string(),
                    filename: entry.headers.filename,
                    content_type: entry.headers.content_type.unwrap(),
                    data: buf
                };

                form_data.files.push(file_part);
            }
        }

        form_data
    }
}
