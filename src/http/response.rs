use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use super::http_code::StatusCode;

pub struct Response {
    pub status_code: StatusCode,
    pub headers: HashMap<String, String>,
    pub data_length: Option<usize>,
    pub data: Vec<u8>,
}

impl Response {
    pub fn new(status_code: StatusCode, headers: HashMap<String, String>, data_length: Option<usize>, data: Vec<u8>) -> Response {
        Response {
            status_code: status_code,
            headers: headers,
            data_length: data_length,
            data: data,
        }
    }

    pub fn empty<S>(status_code: S) -> Response
        where S: Into<StatusCode>
    {
        Response::new(
            status_code.into(),
            HashMap::new(),
            Some(0),
            Vec::new(),
        )
    }

    pub fn from_data<D>(data: D) -> Response
        where D: Into<Vec<u8>>
    {
        let data = data.into();
        let data_len = data.len();

        Response::new(
            StatusCode(200),
            HashMap::new(),
            Some(data_len),
            data,
        )
    }

    pub fn from_file(mut file: File) -> Response {
        let file_size = file.metadata().ok().map(|v| v.len() as usize);

        let mut data: Vec<u8> = Vec::new();
        file.read_to_end(&mut data).unwrap();

        Response::new(
            StatusCode(200),
            HashMap::new(),
            file_size,
            data,
        )
    }

    pub fn from_string<S>(string: S) -> Response
        where S: Into<String>
    {
        let string = string.into();
        let data_len = string.len();

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_owned(), "text/plain; charset=UTF-8".to_owned());

        Response::new(
            StatusCode(200),
            headers,
            Some(data_len),
            string.into(),
        )
    }

    pub fn with_code(mut self, code: u16) -> Response {
        self.status_code = code.into();
        self
    }

    pub fn with_header(mut self, header: (String, String)) -> Response {
        self.headers.insert(header.0, header.1);
        self
    }
}
