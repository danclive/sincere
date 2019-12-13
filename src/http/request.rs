use std::collections::HashMap;

use hyper::header::CONTENT_TYPE;
use hyper::{self, HeaderMap, Method, Uri, Version};
use hyper::body::Bytes;
use http::request::Parts;
use mime::{self, Mime};
use serde::de::DeserializeOwned;
use serde_json;

use super::plus::server::FilePart;
use crate::error::Result;
use crate::util::url;

#[derive(Debug)]
pub struct Request {
    uri: Uri,
    method: Method,
    version: Version,
    headers: HeaderMap,
    params: HashMap<String, String>,
    querys: Vec<(String, String)>,
    posts: Vec<(String, String)>,
    files: Vec<FilePart>,
    body: Bytes
}

impl Request {
    pub(crate) fn from_hyper_request(parts: Parts, body: Bytes) -> Request {
        let mut request = Request {
            uri: parts.uri,
            method: parts.method,
            version: parts.version,
            headers: parts.headers,
            params: HashMap::new(),
            querys: Vec::new(),
            posts: Vec::new(),
            files: Vec::new(),
            body
        };

        request.parse_query();
        request.parse_post();

        request
    }

    #[inline]
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    #[inline]
    pub fn param(&self, key: &str) -> Option<String> {
        self.params.get(key).map(|m| m.to_string())
    }

    #[inline]
    pub fn params(&mut self) -> &mut HashMap<String, String> {
        &mut self.params
    }

    #[inline]
    pub fn query(&self, key: &str) -> Option<String> {
        self.querys
            .iter()
            .find(|&&(ref k, _)| k == key)
            .map(|&(_, ref v)| v.to_string())
    }

    #[inline]
    pub fn querys(&self) -> &Vec<(String, String)> {
        &self.querys
    }

    #[inline]
    pub fn post(&self, key: &str) -> Option<String> {
        self.posts
            .iter()
            .find(|&&(ref k, _)| k == key)
            .map(|&(_, ref v)| v.to_string())
    }

    #[inline]
    pub fn posts(&self) -> &Vec<(String, String)> {
        &self.posts
    }

    pub fn header(&self, name: &str) -> Option<String> {
        if let Some(value) = self.headers.get(name) {
            let value = String::from_utf8_lossy(value.as_bytes());
            return Some(value.to_string());
        }

        None
    }

    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    #[inline]
    pub fn content_type(&self) -> Option<Mime> {
        if let Some(value) = self.headers.get(CONTENT_TYPE) {
            if let Ok(value) = value.to_str() {
                if let Ok(mime) = value.parse::<Mime>() {
                    return Some(mime);
                }
            }
        }

        None
    }

    #[inline]
    fn parse_query(&mut self) {
        let url = match self.uri().query() {
            Some(url) => url.to_owned(),
            None => return,
        };

        self.querys = url::from_str::<Vec<(String, String)>>(&url).unwrap_or_default();
    }

    #[inline]
    fn parse_post(&mut self) {
        let content_type = match self.content_type() {
            Some(c) => c.to_owned(),
            None => return,
        };

        if content_type == mime::APPLICATION_WWW_FORM_URLENCODED {
            let params = String::from_utf8_lossy(&self.body);
            self.posts = url::from_str::<Vec<(String, String)>>(&params).unwrap_or_default();
        } else if content_type.type_() == mime::MULTIPART
            && content_type.subtype() == mime::FORM_DATA
        {
            let form_data = self.parse_formdata();

            if let Some(form_data) = form_data {
                self.posts = form_data.fields;
                self.files = form_data.files;
            }
        }
    }

    #[inline]
    pub fn has_file(&self) -> bool {
        if self.files.len() > 0 {
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn files(&self) -> &Vec<FilePart> {
        &self.files
    }

    #[inline]
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    #[inline]
    pub fn bind_json<D: DeserializeOwned>(&mut self) -> Result<D> {
        Ok(serde_json::from_slice(&self.body())?)
    }
}
