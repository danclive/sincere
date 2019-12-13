//! App context.
use hyper;
use hyper::body::Bytes;
use http::request::Parts;

use nson::Message;

use super::App;
use crate::http::Request;
use crate::http::Response;

/// App context.
pub struct Context<'a> {
    /// app container reference
    pub app: &'a App,
    /// http request
    pub request: Request,
    /// http response
    pub response: Response,
    /// contexts key-value container
    pub contexts: Message,
    stop: bool,
}

impl<'a> Context<'a> {
    pub(crate) fn new(app: &App, parts: Parts, body: Bytes) -> Context {
        let request = Request::from_hyper_request(parts, body);
        let response = Response::empty(200);

        Context {
            app: app,
            request: request,
            response: response,
            contexts: Message::new(),
            stop: false,
        }
    }
    /// Stop the handle to continue.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sincere::App;
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
    ///             context.contexts.insert("id".to_owned(), token.clone());
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
    ///     let token = context.contexts.get_str("token").unwrap();
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
}
