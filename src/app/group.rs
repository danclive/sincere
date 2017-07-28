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

    pub fn get<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add("GET", pattern, handle)
    }

    pub fn post<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add("POST", pattern, handle)
    }

    pub fn put<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add("PUT", pattern, handle)
    }

    pub fn delete<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add("DELETE", pattern, handle)
    }

    pub fn option<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add("OPTION", pattern, handle)
    }

    pub fn head<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add("HEAD", pattern, handle)
    }

    pub fn before<H>(&mut self, handle: H)
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.before.push(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn after<H>(&mut self, handle: H)
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.after.push(Middleware {
            inner: Box::new(handle),
        });
    }
}