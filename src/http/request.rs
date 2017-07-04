use std::collections::HashMap;
use std::net::SocketAddr;

use serde::de::DeserializeOwned;
use serde_json;

use super::http_method::Method;
use util::url;
use error::Result;

pub struct Request {
    method: Method,
    path: String,
    version: String,
    headers: HashMap<String, String>,
    params: HashMap<String, String>,
    querys: HashMap<String, String>,
    remote_addr: SocketAddr,
    data: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, path: String, version: String, headers: HashMap<String, String>, remote_addr: SocketAddr, data: Vec<u8>) -> Request {
        Request {
            method: method,
            path: path,
            version: version,
            headers: headers,
            params: HashMap::new(),
            querys: HashMap::new(),
            remote_addr: remote_addr,
            data: data,
        }
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn headers(&mut self) -> &mut HashMap<String, String> {
        &mut self.headers
    }

    pub fn get_header<'a, S>(&self, key: S) -> Option<String>
        where S: Into<&'a str>
    {
        self.headers.get(key.into()).map(|v| v.to_string())
    }

    pub fn remote_addr(&self) -> &SocketAddr {
        &self.remote_addr
    }

    pub fn data_length(&self) -> usize {
        self.data.len()
    }

    pub fn data(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    pub fn params(&mut self) -> &mut HashMap<String, String> {
        &mut self.params
    }

    pub fn get_param<'a, S>(&self, key: S) -> Option<String>
        where S: Into<&'a str>
    {
        self.params.get(key.into()).map(|v| v.to_string())
    }

    fn decode_query(&mut self) {
        if self.querys.len() == 0 {
            let url: String = self.path.find('?').map_or("".to_owned(), |pos| self.path[pos + 1..].to_owned());
            if url.len() > 0 {
                self.querys = url::from_str::<HashMap<String, String>>(&url).unwrap();
            }
        }
    }

    pub fn querys(&mut self) -> &mut HashMap<String, String> {
        self.decode_query();

        &mut self.querys
    }

    pub fn get_query<'a, S>(&mut self, key: S) -> Option<String>
        where S: Into<&'a str>
    {
        self.decode_query();

        self.querys.get(key.into()).map(|v| v.to_string())
    }

    pub fn bind_json<D: DeserializeOwned>(&self) -> Result<D> {
        Ok(serde_json::from_slice(&self.data)?)
    }
}

/*
trait VecFind {
    fn find(&self, key: &str) -> Option<&str>;
}

impl VecFind for Vec<(String, String)> {
    fn find(&self, key: &str) -> Option<&str> {
        self.iter().find(|&&(ref k, _)| k == key ).map(|&(_, ref v)| &**v)
    }
}
*/
