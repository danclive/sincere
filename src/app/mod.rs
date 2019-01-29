//! App container.
use hyper::{Request, Response, Body};
use hyper::Method;

pub use self::route::Route;
pub use self::group::Group;
use self::middleware::Middleware;
use self::context::Context;
pub use self::run::run;

#[macro_use]
mod macros;
mod route;
mod group;
pub mod middleware;
pub mod context;
mod run;

pub type Handle = Fn(&mut Context) + Send + Sync + 'static;

/// App container.
///
/// ```no_run
/// use sincere::App;
/// use sincere::app::run;
/// use lazy_static::lazy_static;
///
/// lazy_static! {
///     static ref APP: App = start();
/// }
///
/// fn start() -> App {
///     let mut app = App::new();
///
///     app.get("/", |context| {
///         context.response.from_text("Hello world!").unwrap();
///     });
///
///     app
/// }
///
/// fn main() {
///     run("0.0.0.0:10001", 4, &APP).unwrap();    
/// }
/// ```
///
#[derive(Default)]
pub struct App {
    groups: Vec<Group>,
    begin: Vec<Middleware>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
    finish: Vec<Middleware>,
    not_found: Option<Middleware>
}

impl App {
    /// Create an app container.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::App;
    ///
    /// let app = App::new();
    /// ```
    ///
    pub fn new() -> App {
        App {
            groups: vec![Group::new("")],
            begin: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            finish: Vec::new(),
            not_found: None
        }
    }

    /// Add route handle to app.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::App;
    /// use sincere::http::Method;
    ///
    /// let mut app = App::new();
    ///
    /// app.add(Method::GET, "/", |context| {
    ///     context.response.from_text("Get method!").unwrap();
    /// });
    /// ```
    pub fn add<H>(&mut self, method: Method, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.groups.get_mut(0).unwrap().add(method, pattern, handle)
    }

    route!(
        /// Add route handle to app with GET method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.get("/", |context| {
        ///    context.response.from_text("Get method!").unwrap();
        /// });
        /// ```
        get
    );

    route!(
        /// Add route handle to app with PUT method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.put("/", |context| {
        ///    context.response.from_text("Put method!").unwrap();
        /// });
        /// ```
        put
    );

    route!(
        /// Add route handle to app with POST method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.post("/", |context| {
        ///    context.response.from_text("Post method!").unwrap();
        /// });
        /// ```
        post
    );

    route!(
        /// Add route handle to app with HEAD method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.head("/", |context| {
        ///    // no body?
        ///    // context.response.from_text("Head method!").unwrap();
        /// });
        /// ```
        head
    );

    route!(
        /// Add route handle to app with PATCH method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.patch("/", |context| {
        ///    context.response.from_text("Patch method!").unwrap();
        /// });
        /// ```
        patch
    );

    route!(
        /// Add route handle to app with TRACE method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.trace("/", |context| {
        ///    context.response.from_text("Trace method!").unwrap();
        /// });
        /// ```
        trace
    );

    route!(
        /// Add route handle to app with DELETE method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.delete("/", |context| {
        ///    context.response.from_text("Delete method!").unwrap();
        /// });
        /// ```
        delete
    );

    route!(
        /// Add route handle to app with OPTIONS method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.options("/", |context| {
        ///    context.response.from_text("Options method!").unwrap();
        /// });
        /// ```
        options
    );

    route!(
        /// Add route handle to app with CONNECT method.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        /// use sincere::http::Method;
        ///
        /// let mut app = App::new();
        ///
        /// app.connect("/", |context| {
        ///    context.response.from_text("Connect method!").unwrap();
        /// });
        /// ```
        connect
    );

    /// Mount router group to app.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::App;
    ///
    /// let mut app = App::new();
    ///
    /// app.mount("/app", |group| {
    ///
    ///     group.get("/", |context| {
    ///         context.response.from_text("Get method!").unwrap();
    ///     });
    ///
    ///     group.post("/", |context| {
    ///         context.response.from_text("Post method!").unwrap();
    ///     });
    ///
    /// });
    /// ```
    pub fn mount<F>(&mut self, prefix: &str, func: F) -> &mut App
        where F: Fn(&mut Group)
    {
        let mut group = Group::new(prefix); 

        func(&mut group);
        
        self.groups.push(group);
        self
    }

    /// Mount router group to app.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::App;
    /// use sincere::app::Group;
    ///
    /// let mut group = Group::new("/app");
    ///
    /// group.get("/", |context| {
    ///     context.response.from_text("Get method!").unwrap();
    /// });
    ///
    /// group.post("/", |context| {
    ///     context.response.from_text("Post method!").unwrap();
    /// });
    ///
    /// let mut app = App::new();
    ///
    /// app.mount_group(group);
    ///
    pub fn mount_group(&mut self, group: Group) -> &mut App {
        self.groups.push(group);
        self
    }

    middleware!(
        /// Add `begin handle` to app.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        ///
        /// let mut app = App::new();
        ///
        /// app.begin(|context| {
        ///     context.response.from_text("begin!").unwrap();
        /// });
        /// ```
        begin
    );

    middleware!(
        /// Add `before handle` to app.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        ///
        /// let mut app = App::new();
        ///
        /// app.before(|context| {
        ///     context.response.from_text("before!").unwrap();
        /// });
        /// ```
        before
    );

    middleware!(
        /// Add `after handle` to app.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        ///
        /// let mut app = App::new();
        ///
        /// app.after(|context| {
        ///     context.response.from_text("after!").unwrap();
        /// });
        /// ```
        after
    );

    middleware!(
        /// Add `finish handle` to app.
        ///
        /// # Examples
        ///
        /// ```
        /// use sincere::App;
        ///
        /// let mut app = App::new();
        ///
        /// app.finish(|context| {
        ///     context.response.from_text("finish!").unwrap();
        /// });
        /// ```
        finish
    );

    /// Use middleware
    ///
    /// # Example
    ///
    /// ```
    /// use sincere::App;
    ///
    /// let mut app = App::new();
    ///
    /// app.middleware(|app| {
    ///     
    ///     app.begin(|context| {
    ///         context.response.from_text("Hello world!").unwrap();
    ///     });
    ///
    ///     app.finish(|context| {
    ///         context.response.from_text("Hello world!").unwrap();
    ///     });
    ///
    /// });
    /// ```
    pub fn middleware<F>(&mut self, func: F) -> &mut App
        where F: Fn(&mut App)
    {
        func(self);
        self
    }

    /// Add `not-found handle` to app.
    ///
    /// # Examples
    ///
    /// ```
    /// use sincere::App;
    ///
    /// let mut app = App::new();
    ///
    /// app.not_found(|context| {
    ///     context.response.status_code(404).from_text("Not Found!").unwrap();
    /// });
    /// ```
    pub fn not_found<H>(&mut self, handle: H)
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.not_found = Some(Middleware {
            inner: Box::new(handle),
        });
    }
        /// handle
    fn handle(&self, request: Request<Body>) -> Response<Body> {

        let mut context = Context::new(self, request);

        let mut route_found = false;

        for begin in self.begin.iter() {
            begin.execute_always(&mut context);
        }

        if context.next() {
            let path = {
                let path = context.request.uri().path();
                if path != "/" {
                    path.trim_end_matches('/').to_owned()
                } else {
                    path.to_owned()
                }
            };

            'outer: for group in self.groups.iter() {

                if let Some(routes) = group.routes.get(context.request.method()) {

                    for route in routes.iter() {
                        if let Some(ref regex) = route.regex {
                            let caps = regex.captures(&path);

                            if let Some(caps) = caps {
                                route_found = true;

                                let matches = route.path();

                                for (key, value) in matches.iter() {
                                    context.request.params().insert(key.to_owned(), caps.get(*value).unwrap().as_str().to_owned());
                                }
                            }
                        } else {
                            let pattern = {
                                let pattern = route.pattern();
                                if pattern != "/" {
                                    pattern.trim_end_matches('/').to_owned()
                                } else {
                                    pattern.to_owned()
                                }
                            };

                            if pattern == path {
                                route_found = true;
                            }
                        }

                        if route_found {

                            for before in self.before.iter() {
                                before.execute(&mut context);
                            }

                            for before in group.before.iter() {
                                before.execute(&mut context);
                            }

                            route.execute(&mut context);

                            for after in group.after.iter() {
                                after.execute(&mut context);
                            }

                            for after in self.after.iter() {
                                after.execute(&mut context);
                            }

                            break 'outer;
                        }
                    }
                }
            }

            if !route_found {
                if let Some(ref not_found) = self.not_found {
                    not_found.execute(&mut context);
                } else {
                    context.response.status_code(404).from_text("Not Found").unwrap();
                }
            }
        }

        for finish in self.finish.iter() {
            finish.execute_always(&mut context);
        }

        context.finish()
    }
}
