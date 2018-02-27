use super::route::Route;
use super::context::Context;
use super::middleware::Middleware;

use hyper::Method;

pub struct Group {
    pub routes: Vec<Route>,
    prefix: String,
    pub before: Vec<Middleware>,
    pub after: Vec<Middleware>,
}

impl Group {
    pub fn new(prefix: &str) -> Group {
        Group {
            routes: Vec::new(),
            prefix: prefix.to_owned(),
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    fn add<H>(&mut self, method: Method, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        let route = Route::new(
            method, 
            self.prefix.clone() + pattern,
            Box::new(handle),
        );

        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    route!(get);
    route!(put);

    route!(post);
    route!(head);

    route!(patch);
    route!(trace);

    route!(delete);

    route!(options);
    route!(connect);

    middleware!(before);
    middleware!(after);
}
