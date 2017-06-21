use std::sync::{Arc, Mutex};

use regex::Regex;

use server::Server;
use server::Stream;

use http::Http;
use http::Request;
use http::Response;

use self::route::Route;
pub use self::route::Group;
use error::Result;

mod route;

pub type Handle = Fn(&mut Request, &mut Response) + Send + Sync + 'static;

pub struct Micro {
    routes: Vec<Route>,
    begin: Vec<Middleware>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
    finish: Vec<Middleware>,
    not_found: Option<Middleware>,
}

impl Micro {
    pub fn new() -> Micro {
        Micro {
            routes: Vec::new(),
            begin: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            finish: Vec::new(),
            not_found: None,
        }
    }

    fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        let route = Route::new(
            method.parse().unwrap(), 
            pattern.into(), 
            Box::new(handle),
        );

        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    pub fn get<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("GET", pattern, handle)
    }

    pub fn post<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("POST", pattern, handle)
    }

    pub fn put<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("PUT", pattern, handle)
    }

    pub fn delete<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("DELETE", pattern, handle)
    }

    pub fn option<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("OPTION", pattern, handle)
    }

    pub fn head<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.add("HEAD", pattern, handle)
    }

    pub fn mount(&mut self, mut group: Group) {
        self.routes.append(group.routes.as_mut());
    }

    pub fn begin<H>(&mut self, handle: H)
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.begin.push(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn before<H>(&mut self, handle: H)
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.before.push(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn after<H>(&mut self, handle: H)
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.after.push(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn finish<H>(&mut self, handle: H)
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.finish.push(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn not_found<H>(&mut self, handle: H)
        where H: Fn(&mut Request, &mut Response) + Send + Sync + 'static
    {
        self.not_found = Some(Middleware {
            inner: Box::new(handle),
        });
    }

    pub fn handle(&self, stream: Arc<Mutex<Stream>>) {
        let mut http = Http::new(stream);
        
        let mut request = http.decode();
        let mut response = Response::empty(200);

        let mut route_found = false;

        for begin in self.begin.iter() {         
            begin.execute(&mut request, &mut response);
        }

        for route in self.routes.iter() {
            if route.method() != request.method() {
                continue;
            }

            let path = {
                let path = request.path();
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
                        request.params().insert(key.to_owned(), caps.get(*value).unwrap().as_str().to_owned());
                    }
                }
            } else {
                if pattern == path {
                    route_found = true;
                }
            }

            if route_found {
                
                for before in self.before.iter() {
                    before.execute(&mut request, &mut response);
                }

                if !response.is_stop() {
                    route.execute(&mut request, &mut response);
                }

                for after in self.after.iter() {
                    after.execute(&mut request, &mut response);
                }

                break;
            }
        }

        if !route_found {
            if let Some(ref not_found) = self.not_found {
                not_found.execute(&mut request, &mut response);
            } else {
                response.status(404).from_text("Not Found");
            }
        }

        for finish in self.finish.iter() {
            finish.execute(&mut request, &mut response);
        }

        http.encode(response);
    }

    pub fn run(self, addr: &str) -> Result<()> {

        let mut server = Server::new(addr).unwrap();

        server.run(Box::new(move |stream| {
            self.handle(stream);
        }))?;

        Ok(())
    }

    pub fn run_tls(self, addr: &str, cert: &str, private_key: &str) -> Result<()> {
        let mut server = Server::new(addr).unwrap();

        server.run_tls(Box::new(move |stream| {
            self.handle(stream);
        }), cert, private_key)?;

        Ok(())
    }
}

struct Middleware {
    inner: Box<Handle>,
}

impl Middleware {
    fn execute(&self, request: &mut Request, response: &mut Response) {
        (self.inner)(request, response);
    }
}
