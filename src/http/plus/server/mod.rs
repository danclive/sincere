use std::path::PathBuf;
use std::fs;
use std::io::Write;

use mime;

use crate::error::Result;
use crate::http::request::Request;

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

            return FormData::parse(self.body(), boundary)
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
    pub filename: String,
    pub content_type: mime::Mime,
    pub data: Vec<u8>,
}

impl FilePart {
    pub fn save_file<P: Into<PathBuf>>(&mut self, path: P) -> Result<PathBuf> {
        let mut path_buf = path.into();

        path_buf.push(self.filename.clone());

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

    pub fn parse(body: &[u8], boundary: &str) -> Option<FormData> {
        let boundary = "--".to_owned() + boundary;

        let mut form_data = FormData::new();

        {
            if !has_boundary(body, &boundary) {
                return None
            }

            if body.len() <= boundary.len() + 2 {
                return None
            }

            let mut part: Vec<(usize, usize)> = Vec::new(); 

            let mut cursor = boundary.len() + 2;

            loop {
                match twoway::find_bytes(&body[cursor..], boundary.as_bytes()) {
                    Some(index) => {
                        if index == 0 {
                            break;
                        }

                        if &body[cursor + index - 2..cursor + index] != b"\r\n" {
                            return None
                        }

                        part.push((cursor, cursor + index - 2));

                        cursor = cursor + boundary.len() + 2 + index;

                        if cursor > body.len() {
                            return None;
                        }
                    },
                    None => {
                        if cursor == body.len() && &body[cursor - 2..cursor] == b"--" {
                            break;
                        }

                        if cursor == body.len() - 2 && &body[cursor - 2..cursor] == b"--" {
                            break;
                        }

                        return None;
                    }
                }
            }

            for (start, end) in part {
                let mut headers = [httparse::EMPTY_HEADER; 4];
                match httparse::parse_headers(&body[start..end], &mut headers) {
                    Ok(httparse::Status::Complete((index, raw_headers))) => {
                        if let Some(value) = get_value_from_header(raw_headers, "Content-Disposition") {
                            let ss: Vec<&str> = value.split(";").collect();

                            let mut name = "";
                            let mut filename = None;

                            for s in ss {
                                let s = s.trim();
                                if s.starts_with("name") {
                                    name = &s[6..s.len()-1];
                                } else if s.starts_with("filename") {
                                    filename = Some(&s[10..s.len()-1]);
                                }
                            }

                            // is file
                            if let Some(filename) = filename {
                                let content_type = {
                                    if let Some(value) = get_value_from_header(raw_headers, "Content-Type") {
                                        value.parse().unwrap_or(mime::APPLICATION_OCTET_STREAM)
                                    } else {
                                        mime::APPLICATION_OCTET_STREAM
                                    }
                                };

                                let file_part = FilePart {
                                    name: name.to_string(),
                                    filename: filename.to_owned(),
                                    content_type: content_type,
                                    data: body[start + index..end].to_vec()
                                };

                                form_data.files.push(file_part);


                            } else {
                                form_data.fields.push(
                                    (name.to_string(), String::from_utf8_lossy(&body[start + index..end]).into_owned())
                                );
                            }
                        } else {
                            return None
                        }
                    },
                    Ok(httparse::Status::Partial) => {
                        return None
                    },
                    Err(_) => {
                        return None
                    }
                }
            }
        }

        Some(form_data)
    }
}

fn has_boundary(body: &[u8], boundary: &str) -> bool {
    match twoway::find_bytes(body, boundary.as_bytes()) {
        Some(index) => {
            if index == 0 {
                return true;
            } else {
                return false;
            }
        },
        None => return false
    }
}

fn get_value_from_header<'a>(headers: &'a [httparse::Header], key: &str) -> Option<String> {
    for header in headers {
        if header.name == key {
            return Some(String::from_utf8_lossy(header.value).to_string())
        }
    }

    None
}
