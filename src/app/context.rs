use std::collections::HashMap;
use std::time::Instant;

use hyper;

use super::App;
use http::Request;
use http::Response;

pub struct Context<'a> {
    pub app: &'a App,
    pub request: Request,
    pub response: Response,
    pub contexts: HashMap<String, Value>,
    stop: bool
}

impl<'a> Context<'a> {

    pub fn new(app: &App, hyper_request: hyper::Request) -> Context {
        let request = Request::from_hyper_request(hyper_request);
        let response = Response::empty(200);

        Context {
            app: app,
            request: request,
            response: response,
            contexts: HashMap::new(),
            stop: false
        }
    }

    pub fn stop(&mut self) {
        self.stop = true;
    }

    pub fn next(&self) -> bool {
        !self.stop
    }

    pub fn finish(self) -> hyper::Response {
        self.response.raw_response()
    }
}

pub enum Value {
    String(String),
    Int32(i32),
    Int64(i64),
    Usize(usize),
    Isize(isize),
    Double(f64),
    Array(Vec<Value>),
    Map(HashMap<Value, Value>),
    Boolean(bool),
    Binary(Vec<u8>),
    Instant(Instant)
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Value::String(ref s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Value::Int32(ref i) => Some(*i),
            _ => None
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Value::Int64(ref i) => Some(*i),
            _ => None
        }
    }

    pub fn as_usize(&self) -> Option<usize> {
        match *self {
            Value::Usize(ref i) => Some(*i),
            _ => None
        }
    }

    pub fn as_isize(&self) -> Option<isize> {
        match *self {
            Value::Isize(ref i) => Some(*i),
            _ => None
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Value::Double(ref i) => Some(*i),
            _ => None
        }
    }

    pub fn as_vec(&self) -> Option<&Vec<Value>> {
        match *self {
            Value::Array(ref i) => Some(i),
            _ => None
        }
    }

    pub fn as_map(&self) -> Option<&HashMap<Value, Value>> {
        match *self {
            Value::Map(ref i) => Some(i),
            _ => None
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Value::Boolean(ref i) => Some(*i),
            _ => None
        }
    }

    pub fn as_binary(&self) -> Option<&Vec<u8>> {
        match *self {
            Value::Binary(ref i) => Some(i),
            _ => None
        }
    }

    pub fn as_instant(&self) -> Option<&Instant> {
        match *self {
            Value::Instant(ref i) => Some(i),
            _ => None
        }
    }
}
