use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use serde::Serialize;
use serde_json;

use hyper;
//use hyper::header::ContentLength;

use super::status_code::StatusCode;
use crate::error::Result;

#[derive(Debug)]
pub struct Response {
    status_code: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl Response {
    pub fn new(
        status_code: StatusCode,
        headers: HashMap<String, String>,
        data: Vec<u8>,
    ) -> Response {
        Response {
            status_code: status_code,
            headers: headers,
            body: data,
        }
    }

    pub fn empty<S>(status_code: S) -> Response
    where
        S: Into<StatusCode>,
    {
        Response::new(status_code.into(), HashMap::new(), Vec::new())
    }

    pub fn from_data<C, D>(&mut self, content_type: C, data: D) -> Result<&mut Response>
    where
        C: Into<String>,
        D: Into<Vec<u8>>,
    {
        let data = data.into();

        self.headers
            .insert("Content-Type".to_owned(), content_type.into());
        self.body = data;

        Ok(self)
    }

    pub fn from_file<C>(&mut self, content_type: C, mut file: File) -> Result<&mut Response>
    where
        C: Into<String>,
    {
        //let file_size = file.metadata().ok().map(|v| v.len() as usize);
        let mut data: Vec<u8> = Vec::new();
        file.read_to_end(&mut data)?;

        self.headers
            .insert("Content-Type".to_owned(), content_type.into());
        self.body = data;

        Ok(self)
    }

    pub fn from_text<S>(&mut self, string: S) -> Result<&mut Response>
    where
        S: Into<String>,
    {
        let string = string.into();

        self.headers.insert(
            "Content-Type".to_owned(),
            "text/plain; charset=UTF-8".to_owned(),
        );
        self.body = string.into();

        Ok(self)
    }

    pub fn from_html<S>(&mut self, string: S) -> Result<&mut Response>
    where
        S: Into<String>,
    {
        let string = string.into();

        self.headers.insert(
            "Content-Type".to_owned(),
            "text/html; charset=UTF-8".to_owned(),
        );
        self.body = string.into();

        Ok(self)
    }

    pub fn from_json<S: Serialize>(&mut self, value: S) -> Result<&mut Response> {
        let data = serde_json::to_vec(&value)?;

        self.headers.insert(
            "Content-Type".to_owned(),
            "application/json; charset=UTF-8".to_owned(),
        );
        self.body = data;

        Ok(self)
    }

    #[inline]
    pub fn status_code(&mut self, code: u16) -> &mut Response {
        self.status_code = code.into();
        self
    }

    #[inline]
    pub fn get_status_code(&self) -> u16 {
        self.status_code.0
    }

    #[inline]
    pub fn header<S>(&mut self, header: (S, S)) -> &mut Response
    where
        S: Into<String>,
    {
        self.headers.insert(header.0.into(), header.1.into());
        self
    }

    #[inline]
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    #[inline]
    pub fn get_headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    #[inline]
    pub(crate) fn raw_response(self) -> hyper::Response<hyper::Body> {
        let mut header_builder = hyper::Response::builder()
            .status(self.get_status_code());

        for (key, value) in self.headers.iter() {
            header_builder = header_builder.header(&**key, &**value);
        }

        header_builder.body(hyper::Body::from(self.body)).unwrap()
    }
}
