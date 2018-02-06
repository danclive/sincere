use std::str::FromStr;
use std::collections::HashMap;
use std::io::Read;

use serde::de::DeserializeOwned;
use serde_json;

use mime;

use fastcgi;
use text;
use util::url;

use error::Result;

use super::method::Method;

pub struct Request {
    raw: fastcgi::Request,
    uri: String,
    method: Method,
    params: HashMap<String, String>,
    querys: HashMap<String, String>,
    posts: HashMap<String, String>,
    body: Vec<u8>
}

impl Request {
    pub fn from_fastcgi(raw_request: fastcgi::Request) -> Request {
        let request_uri = raw_request.param("REQUEST_URI").unwrap_or(String::default());

        let method = raw_request.param("REQUEST_METHOD").or(raw_request.param("X-HTTP-METHOD-OVERRIDE")).unwrap_or_default();

        let method = Method::from_str(&method).unwrap();

        let mut request = Request {
            raw: raw_request,
            uri: request_uri,
            method: method,
            params: HashMap::new(),
            querys: HashMap::new(),
            posts: HashMap::new(),
            body: Vec::new()
        };

        request.parse_query();
        request.parse_post();

        request
    }

    #[inline]
    pub fn uri(&self) -> &str {
        &self.uri
    }

    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    #[inline]
    pub fn param(&self, key: &str) -> Option<String> {
        self.params.get(key).map(|m| m.to_owned() )
    }

    #[inline]
    pub fn params(&mut self) -> &mut HashMap<String, String> {
        &mut self.params
    }

    #[inline]
    pub fn query(&self, key: &str) -> Option<String> {
        self.querys.get(key).map(|q| q.to_owned() )
    }

    #[inline]
    pub fn querys(&mut self) -> &HashMap<String, String> {
        &mut self.querys
    }

    #[inline]
    pub fn header(&self, key: &str) -> Option<String> {
        let key = key.to_uppercase().replace("-", "_");

        self.raw.param(&key).or(self.raw.param(&("HTTP_".to_owned() + &key)))
    }

    pub fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        for (key, value) in self.raw.params() {
            if key.starts_with("HTTP_") {
                let (_, key) = key.split_at(5);
                let key = text::unwords(key, "_");
                headers.insert(key, value.to_owned());
            } else {
                let key = text::unwords(key, "_");
                headers.insert(key, value.to_owned());
            }
        }

        headers
    }

    #[inline]
    pub fn content_type(&self) -> Option<String> {
        self.header("CONTENT_TYPE")
    }

    #[inline]
    pub fn content_length(&self) -> usize {
        match self.header("CONTENT_LENGTH") {
            Some(content_length) => {
                content_length.parse().unwrap_or(0)
            }
            None => 0
        }
    }

    #[inline]
    fn parse_query(&mut self) {
        let uri: String = self.uri.find('?').map_or("".to_owned(), |pos| self.uri[pos + 1..].to_owned());

        if uri.len() > 0 {
            self.querys = url::from_str::<HashMap<String, String>>(&uri).unwrap_or_default();
        }
    }

    fn parse_post(&mut self) {

        match self.content_type() {
            Some(content_type) => {
                match content_type.parse::<mime::Mime>() {
                    Ok(mime) => {
                        if mime.type_() == mime::APPLICATION && mime.subtype() == mime::WWW_FORM_URLENCODED {
                            self.body();

                            let params = String::from_utf8_lossy(&self.body);
                            self.posts = url::from_str::<HashMap<String, String>>(&params).unwrap_or_default();

                            return
                        }
                    },
                    Err(_) => ()
                }
            }
            None => ()
        }
    }

    #[inline]
    pub fn body(&mut self) -> &mut Vec<u8> {
        if self.body.len() == 0 {
            let length = self.content_length();

            if length > 0 {
                let mut buf: Vec<u8> = Vec::with_capacity(length);
                let _ = self.raw.stdin().read_to_end(&mut buf);
                self.body = buf;
            }
        }

        &mut self.body
    }

    #[inline]
    pub fn raw(&mut self) -> &mut fastcgi::Request {
        &mut self.raw
    }

    #[inline]
    pub fn bind_json<D: DeserializeOwned>(&mut self) -> Result<D> {
        Ok(serde_json::from_slice(&self.body())?)
    }
}
