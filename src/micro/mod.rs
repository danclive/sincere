use regex::Regex;

use server::Server;
use server::Stream;

use http::Http;
use http::Request;
use http::Response;

use self::route::Route;

mod route;

pub struct Micro {
    routes: Vec<Route>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
    finish: Vec<Middleware>,
    not_found: Option<Middleware>,
}

impl Micro {
    pub fn new() -> Micro {
        Micro {
            routes: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            finish: Vec::new(),
            not_found: None,
        }
    }

    pub fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
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

    pub fn handle(&self, stream: Stream) {
        let mut http = Http::new(stream);
        let mut request = http.decode();
        let mut response = Response::empty(200);

        let mut route_found = false;

        for route in &self.routes {
            if route.method != request.method() {
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

            let pattern = route.compilied_pattern.clone();

            if pattern.contains("^") {
                let re = Regex::new(&pattern).unwrap();
                let caps = re.captures(&path);

                if let Some(caps) = caps {
                    route_found = true;

                    let matches = route.paths.clone();

                    for (key, value) in matches.iter() {
                        request.params.insert(key.to_owned(), caps.get(*value).unwrap().as_str().to_owned());
                    }
                }
            } else {
                if pattern == path {
                    route_found = true;
                }
            }

            if route_found {
                
                for before in &self.before {
                    before.execute(&mut request, &mut response);
                }

                route.handle.as_ref()(&mut request, &mut response);

                for after in &self.after {
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

        for finish in &self.finish {
            finish.execute(&mut request, &mut response);
        }

        http.encode(response);
    }

    pub fn run(self, addr: &str) {
        let mut server = Server::new();
        
        server.handle(Box::new(move |stream| {
            self.handle(stream);
        }));

        server.run(addr).unwrap();
    }
}

struct Middleware {
    inner: Box<route::Handle>,
}

impl Middleware {
    fn execute(&self, request: &mut Request, response: &mut Response) {
        self.inner.as_ref()(request, response);
    }
}
