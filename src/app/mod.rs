//! App container.
use std::sync::Arc;
use std::rc::Rc;

use regex::Regex;

use futures::future::{self, Future};
use futures_cpupool::CpuPool;

use hyper::{self, Request, Response, Body, Server};
use hyper::service::{Service, NewService, service_fn};

//use hyper::server::{Http, Request, Response, Service};
use hyper::Method;

use queen_log::color::Print;

use error::Result;
pub use self::route::Route;
pub use self::group::Group;
use self::middleware::Middleware;
use self::context::Context;

#[macro_use]
mod macros;
mod route;
mod group;
pub mod middleware;
pub mod context;

pub type Handle = Fn(&mut Context) + Send + Sync + 'static;

/// App container.
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
/// app.run("127.0.0.1:8000", 20).unwrap();
/// ```
///
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
    /// app.add(Method::Get, "/", |context| {
    ///     context.response.from_text("Get method!").unwrap();
    /// });
    /// ```
    pub fn add<H>(&mut self, method: Method, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        let route = Route::new(
            method,
            pattern.into(), 
            Box::new(handle),
        );

        self.groups.get_mut(0).unwrap().routes.push(route);
        self.groups.get_mut(0).unwrap().routes.last_mut().unwrap()
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
    pub fn mount<F>(&mut self, prefix: &str, func: F)
        where F: Fn(&mut Group)
    {
        let mut group = Group::new(prefix); 

        func(&mut group);
        
        self.groups.push(group)
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
    pub fn mount_group(&mut self, group: Group) {
        self.groups.push(group)
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
    pub fn middleware<F>(&mut self, func: F)
        where F: Fn(&mut App)
    {
        func(self)
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

            'outer: for group in self.groups.iter() {

                for route in group.routes.iter() {

                    if route.method() != context.request.method() {
                        continue;
                    }

                    let path = {
                        let path = context.request.uri().path();
                        if path != "/" {
                            path.trim_right_matches('/').to_owned()
                        } else {
                            path.to_owned()
                        }
                    };

                    let pattern = {
                        let pattern = route.compilied_pattern();
                        if pattern != "/" {
                            pattern.trim_right_matches('/').to_owned()
                        } else {
                            pattern
                        }
                    };

                    if pattern.contains("^") {
                        let re = Regex::new(&pattern).unwrap();
                        let caps = re.captures(&path);

                        if let Some(caps) = caps {
                            route_found = true;

                            let matches = route.path();

                            for (key, value) in matches.iter() {
                                context.request.params().insert(key.to_owned(), caps.get(*value).unwrap().as_str().to_owned());
                            }
                        }
                    } else {
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

    /// Run app with addr and thread_pool size.
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
    /// app.run("127.0.0.1:8000", 20).unwrap();
    /// ```
    pub fn run(self, addr: &str, thread_size: usize) -> Result<()> {

        let app_service = AppService {
            inner: Arc::new(self),
            thread_pool: CpuPool::new(thread_size)
        };

        // let app = Rc::new(app_service);

        let sincere_logo = Print::green(
    r"
     __.._..  . __ .___.__ .___
    (__  | |\ |/  `[__ [__)[__
    .__)_|_| \|\__.[___|  \[___
    "
        );

        println!("{}", sincere_logo);
        println!(
            "    {}{} {} {} {}",
            Print::green("Server running at http://"),
            Print::green(addr),
            Print::green("on"),
            Print::green(thread_size),
            Print::green("threads.")
        );

        let addr = addr.parse().expect("Address is not valid");
        let thread_pool = CpuPool::new(thread_size);
        //let app = Arc::new(self);
        
        // let service = service_fn(|req: Request<Body>| {
        //     Ok(Response::new(Body::from("Hello World")))
        // });

        //let server = Http::new().bind(&addr, move || Ok(app.clone()))?;
        //server.run()?;
        // let new_svc = || {

        // };

        // let f = || {
        //     app_service
        // };
        //let app2 = app.clone();

        let echo2 = move |req: Request<Body>| -> BoxFut {

            //let app = &app2;
            let mut response = Response::new(Body::empty());

            Box::new(future::ok(response))
        };

        let server = Server::bind(&addr).serve(move || app_service);

        Ok(())
    }
}

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;
type FnA = Fn(Request<Body>) -> BoxFut;

// fn echo(req: Request<Body>) -> BoxFut {
//     let mut response = Response::new(Body::empty());

//     Box::new(future::ok(response))
// }

struct AppService {
    inner: Arc<App>,
    thread_pool: CpuPool
}

impl Service for AppService {
    //type Request = Request;
    //type Response = Response;
    //type Error = hyper::Error;
    //type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = Box<Future<Item = Response<Self::ResBody>, Error = Self::Error> + Send>;

    fn call(&mut self, request: Request<Body>) -> Self::Future {

        let app = self.inner.clone();

        let msg = self.thread_pool.spawn_fn(move || {
            let response = app.handle(request);

            Ok(response)
        });

        Box::new(msg)
    }
}
