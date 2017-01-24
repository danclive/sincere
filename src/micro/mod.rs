use std::collections::HashMap;

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
}

impl Micro {
    pub fn new() -> Micro {
        Micro {
            routes: Vec::new(),
        }
    }

    pub fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&Request) -> Response + Send + Sync + 'static
    {
        let route = Route::new(
            method.parse().unwrap(), 
            pattern.into(), 
            Box::new(handle),
        );

        self.routes.push(route);
        self.routes.last_mut().unwrap()
    }

    pub fn handle(&self, stream: Stream) {
        let mut http = Http::new(stream);
        let request = http.decode();

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

            let mut params: HashMap<String, String> = HashMap::new();
            if pattern.contains("^") {
                let re = Regex::new(&pattern).unwrap();
                let caps = re.captures(&path);

                if let Some(caps) = caps {
                    route_found = true;

                    let matches = route.paths.clone();

                    for (key, value) in matches.iter() {
                        params.insert(key.to_owned(), caps.get(*value).unwrap().as_str().to_owned());
                    }
                }
            } else {
                if pattern == path {
                    route_found = true;
                }
            }

            if route_found {
                let ref handle = route.handle;
                let response = handle(&request);
                http.encode(response);
                break;
            }
        }

        if !route_found {
            http.encode(Response::from_string("404"));
        }
    }

    pub fn run(self, addr: &str) {
        let mut server = Server::new();
        
        server.handle(Box::new(move |stream| {
            self.handle(stream);
        }));

        server.run(addr).unwrap();
    }
}
