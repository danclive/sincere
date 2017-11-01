use std::io;
use std::sync::Arc;

use regex::Regex;

use futures::future;
use tokio_service::Service;
use tokio_service::NewService;
use tokio_proto::TcpServer;

use num_cpus;

use http::Http;
use http::Request;
use http::Response;
pub use self::route::Route;
pub use self::group::Group;
pub use self::context::{Context, Value};
use self::middleware::Middleware;

#[macro_use]
mod macros;
mod route;
mod context;
mod group;
mod middleware;

pub type Handle = Fn(&mut Context) + Send + Sync + 'static;

pub struct App {
    groups: Vec<Group>,
    begin: Vec<Middleware>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
    finish: Vec<Middleware>,
    not_found: Option<Middleware>,
}

impl App {
    pub fn new() -> App {
        App {
            groups: vec![Group::new("")],
            begin: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            finish: Vec::new(),
            not_found: None,
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
    route!(post);
    route!(put);
    route!(delete);
    route!(option);
    route!(head);

    pub fn mount(&mut self, group: Group) {
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

    fn handle(&self, mut context: &mut Context) {

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
                        let path = context.request.path();
                        let path = path.find('?').map_or(path.as_ref(), |pos| &path[..pos]);
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
                    context.response.status(404).from_text("Not Found").unwrap();
                }
            }

        }

        for finish in self.finish.iter() {
            finish.execute_always(&mut context);
        }
    }

    pub fn run(self, addr: &str) {
        let addr = addr.parse().unwrap();
        let mut server = TcpServer::new(Http, addr);
        server.threads(num_cpus::get());

        let a = AppServer{
            inner: Arc::new(self)
        };

        server.with_handle(move |_|{
            a.clone()
        });  
    }
}

#[derive(Clone)]
struct AppServer {
    pub inner: Arc<App>
}

impl Service for App {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = future::Ok<Response, io::Error>;

    fn call(&self, request: Self::Request) -> Self::Future {
        let mut context = Context::new(request);

        self.handle(&mut context);

        future::ok(context.response)
    }
}

impl NewService for AppServer {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Instance = Arc<App>;
    fn new_service(&self) -> io::Result<Self::Instance> {
        Ok(self.inner.clone())
    }
}
