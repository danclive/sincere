use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde_json;

use futures::{Future, Stream};

use hyper::{self, Uri, Method, Headers};
use hyper::header::ContentType;

use util::url;

use error::Result;

pub struct Request {
    uri: Uri,
    method: Method,
    headers: Headers,
    params: HashMap<String, String>,
    querys: Vec<(String, String)>,
    posts: Vec<(String, String)>,
    body: Vec<u8>
}

impl Request {
    pub fn from_hyper_request(hyper_request: hyper::Request) -> Request { 
        let (method, uri, _http_version, headers, body) = hyper_request.deconstruct();

        let body = body.concat2().map(|b| b.to_vec() ).wait().unwrap_or_default();

        let mut request = Request {
            uri: uri,
            method: method,
            headers: headers,
            params: HashMap::new(),
            querys: Vec::new(),
            posts: Vec::new(),
            body: body
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
        self.params.get(key).map(|m| m.to_string() )
    }

    #[inline]
    pub fn params(&mut self) -> &mut HashMap<String, String> {
        &mut self.params
    }

    #[inline]
    pub fn query(&self, key: &str) -> Option<String> {
        self.querys.iter().find(|&&(ref k, _)| k == key ).map(|&(_, ref v)| v.to_string())
    }

    #[inline]
    pub fn querys(&self) -> &Vec<(String, String)> {
        &self.querys
    }

    #[inline]
    pub fn post(&self, key: &str) -> Option<&str> {
        self.posts.iter().find(|&&(ref k, _)| k == key ).map(|&(_, ref v)| &**v)
    }

    #[inline]
    pub fn posts(&self) -> &Vec<(String, String)> {
        &self.posts
    }

    pub fn header(&self, name: &str) -> Option<String> {
        match self.headers.get_raw(name) {
            Some(value) => {
                match value.one() {
                    Some(value) => {
                        let value = String::from_utf8_lossy(value);

                        return Some(value.to_string())
                    },
                    None => return None
                }
            }
            None => return None
        };
    }

    #[inline]
    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    #[inline]
    pub fn content_type(&self) -> Option<&ContentType> {
        self.headers.get::<ContentType>()
    }

    #[inline]
    fn parse_query(&mut self) {
        let url = match self.uri().query() {
            Some(url) => url.to_owned(),
            None => return
        };

        self.querys = url::from_str::<Vec<(String, String)>>(&url).unwrap_or_default();
    }

    #[inline]
    fn parse_post(&mut self) {

        let content_type = match self.content_type() {
            Some(c) => c.to_owned(),
            None => return
        };

        if content_type == ContentType::form_url_encoded() {
            let params = String::from_utf8_lossy(&self.body);
            self.posts = url::from_str::<Vec<(String, String)>>(&params).unwrap_or_default();
        }
    }

    #[inline]
    pub fn body(&self) -> &Vec<u8> {
        &self.body
    }

    #[inline]
    pub fn bind_json<D: DeserializeOwned>(&mut self) -> Result<D> {
        Ok(serde_json::from_slice(&self.body())?)
    }
}
