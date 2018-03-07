//! App context.
use std::collections::HashMap;
use std::time::Instant;

use hyper;

use super::App;
use http::Request;
use http::Response;

/// App context.
pub struct Context<'a> {
    /// app container reference
    pub app: &'a App,
    /// http request
    pub request: Request,
    /// http response
    pub response: Response,
    /// contexts key-value container
    pub contexts: HashMap<String, Value>,
    stop: bool
}

impl<'a> Context<'a> {

    pub(crate) fn new(app: &App, hyper_request: hyper::Request) -> Context {
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
    /// Stop the handle to continue.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sincere::App;
    /// use sincere::app::context::Value;
    ///
    /// let mut app = App::new();
    ///
    /// app.get("/", |context| {
    ///    context.response.from_text("Hello world!").unwrap();
    /// });
    ///
    /// app.before(|context| {
    ///     if let Some(token) = context.request.header("Token") {
    ///
    ///         if token == "token" {
    ///             context.contexts.insert("id".to_owned(), Value::String(token.clone()));
    ///         } else {
    ///             context.response.from_text("Token validation failed!").unwrap();
    ///             context.stop();
    ///         }
    ///
    ///     } else {
    ///         context.response.status_code(401);
    ///         context.stop();
    ///     }
    /// });
    ///
    /// app.get("/", |context| {
    ///     let token = context.contexts.get("token").unwrap().as_str().unwrap();
    ///     println!("token is: {:?}", token);
    /// });
    /// ```
    pub fn stop(&mut self) {
        self.stop = true;
    }

    pub(crate) fn next(&self) -> bool {
        !self.stop
    }

    pub(crate) fn finish(self) -> hyper::Response {
        self.response.raw_response()
    }
}

/// Content value
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
