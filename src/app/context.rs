//! App context.
use hyper;

use nson::Object;

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
    pub contexts: Object,
    stop: bool
}

impl<'a> Context<'a> {

    pub(crate) fn new(app: &App, hyper_request: hyper::Request<hyper::Body>) -> Context {
        let request = Request::from_hyper_request(hyper_request);
        let response = Response::empty(200);

        Context {
            app: app,
            request: request,
            response: response,
            contexts: Object::new(),
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

    pub(crate) fn finish(self) -> hyper::Response<hyper::Body> {
        self.response.raw_response()
    }

    /*
    pub fn set<K: Into<String>, V: ToContext>(&mut self, key: K, value: V) {
        let v: Value = value.to_context();
        self.contexts.insert(key.into(), v);
    }

    pub fn get<K: Into<String>, V: FromContext>(&self, key: K) -> Option<V> {
        let value = self.contexts.get(&key.into());

        if value.is_none() {
            return None;
        }

        <V as FromContext>::from_context(value.unwrap())
    }
    */
}

/*
pub trait ToContext {
    fn to_context(self) -> Value;
}

pub trait FromContext: Sized {
    fn from_context(value: &Value) -> Option<Self>;
}

impl ToContext for i32 {
    fn to_context(self) -> Value {
        Value::Int32(self)
    }
}

impl FromContext for i32 {
    fn from_context(value: &Value) -> Option<Self> {
        value.as_i32()
    }
}

impl ToContext for i64 {
    fn to_context(self) -> Value {
        Value::Int64(self)
    }
}

impl FromContext for i64 {
    fn from_context(value: &Value) -> Option<Self> {
        value.as_i64()
    }
}
*/
