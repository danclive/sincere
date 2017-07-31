use super::route::Route;
use super::context::Context;
use super::middleware::Middleware;

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

    fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        let route = Route::new(
            method.parse().unwrap(), 
            self.prefix.clone() + pattern,
            Box::new(handle),
        );

        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    route!(get);
    route!(post);
    route!(put);
    route!(delete);
    route!(option);
    route!(head);

    middleware!(before);
    middleware!(after);
}
