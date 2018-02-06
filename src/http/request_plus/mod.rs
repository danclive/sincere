use std::path::PathBuf;
use std::fs;
use std::io::{self, Read, Write};

use mime;

use super::request::Request;

use error::Result;

use self::multipart::Multipart;

pub mod multipart;
pub mod boundary;
pub mod field;
pub mod save;

impl Request {
    pub fn parse_formdata(&mut self) -> Option<FormData> {
        if let Some(content_type) = self.content_type() {
            if let Ok(mime) = content_type.parse::<mime::Mime>() {
                if mime.type_() == mime::MULTIPART && mime.subtype() == mime::FORM_DATA {
                    let boundary = if let Some(boundary) = mime.get_param(mime::BOUNDARY) {
                        boundary.as_str()
                    } else {
                        return None
                    };

                    let reader = io::Cursor::new(self.body());

                    return Some(FormData::with_body(reader, boundary));
                }
            }
        }

        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FormData {
    pub fields: Vec<(String, String)>,
    pub files: Vec<(String, FilePart)>
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
    pub filename: String,
    pub content_type: mime::Mime,
    pub data: Vec<u8>,
}

impl FilePart {
    pub fn save_file<P: Into<PathBuf>>(&mut self, path: P) -> Result<()> {
        let mut path = path.into();

        path.push(&self.filename);

        let path = path.as_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut open_option = fs::OpenOptions::new();

        open_option.write(true).create(true);

        let mut file = open_option.open(path)?;

        file.write_all(&self.data)?;
        file.flush()?;

        Ok(())
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
                    filename: entry.headers.filename.unwrap_or_default(),
                    content_type: entry.headers.content_type.unwrap(),
                    data: buf
                };

                form_data.files.push((entry.headers.name.to_string(), file_part))
            }
        }

        form_data
    }
}

use rand::Rng;
fn random_alphanumeric(len: usize) -> String {
    ::rand::thread_rng().gen_ascii_chars().take(len).collect()
}
