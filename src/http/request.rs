use std::collections::HashMap;
use std::net::SocketAddr;

use super::http_method::Method;

pub struct Request {
    pub method: Method,
    pub path: String,
    version: String,
    pub headers: HashMap<String, String>,
    pub remote_addr: SocketAddr,
    data: Vec<u8>,
}

impl Request {
    pub fn new(method: Method, path: String, version: String, headers: HashMap<String, String>, remote_addr: SocketAddr, data: Vec<u8>) -> Request {
        Request {
            method: method,
            path: path,
            version: version,
            headers: headers,
            remote_addr: remote_addr,
            data: data,
        }
    }

    pub fn method(&self) -> Method {
        self.method.clone()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn version(&self) -> String {
        self.version.clone()
    }

    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    pub fn remote_addr(&self) -> SocketAddr {
        self.remote_addr.clone()
    }

    pub fn data_length(&self) -> usize {
        self.data.len()
    }

    pub fn data(&self) -> &Vec<u8> {
        self.data.as_ref()
    }
}
