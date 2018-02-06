use std::sync::Arc;
use std::rc::Rc;

use regex::Regex;

use futures::future::Future;
use futures_cpupool::CpuPool;

use hyper::server::{Http, Request, Response, Service};
use hyper;
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

pub struct App {
    groups: Vec<Group>,
    begin: Vec<Middleware>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
    finish: Vec<Middleware>,
    not_found: Option<Middleware>
}

impl App {
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

    fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        let route = Route::new(
            method.parse().unwrap(),
            pattern.into(), 
            Box::new(handle),
        );

        self.groups.get_mut(0).unwrap().routes.push(route);
        self.groups.get_mut(0).unwrap().routes.last_mut().unwrap()
    }

    route!(get);
    route!(put);

    route!(post);
    route!(head);

    route!(delete);

    route!(options);
    route!(connect);

    pub fn mount<F>(&mut self, func: F)
        where F: Fn() -> Group
    {
        let group = func();

        self.groups.push(group)
    }

    middleware!(begin);
    middleware!(before);
    middleware!(after);
    middleware!(finish);

    pub fn use_middleware<F>(&mut self, func: F)
        where F: Fn(&mut App)
    {
        func(self)
    }

    pub fn not_found<H>(&mut self, handle: H)
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.not_found = Some(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn handle(&self, request: Request) -> Response {

        let mut context = Context::new(request);

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

    pub fn run(self, addr: &str, thread_size: usize) -> Result<()> {

        let app_service = AppService {
            inner: Arc::new(self),
            thread_pool: CpuPool::new(thread_size)
        };

        let app = Rc::new(app_service);

        let addr = addr.parse().expect("Address is not valid");
        let server = Http::new().bind(&addr, move || Ok(app.clone()))?;
        server.run()?;

        Ok(())
    }
}

struct AppService {
    inner: Arc<App>,
    thread_pool: CpuPool
}

impl Service for AppService {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, request: Request) -> Self::Future {

        let app = self.inner.clone();

        let msg = self.thread_pool.spawn_fn(move || {
            let response = app.handle(request);

            Ok(response)
        });

        Box::new(msg)
    }
}
