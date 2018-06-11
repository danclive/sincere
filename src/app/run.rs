//! App run.
use futures::future::Future;
use futures_cpupool::CpuPool;

use hyper::{self, Request, Response, Body, Server};
use hyper::service::service_fn;

use queen_log::color::Print;

use error::Result;
use app::App;

pub trait AppHandle {
    fn handle(&self, request: Request<Body>) -> Response<Body>;
}

struct DefaultApp;

impl AppHandle for DefaultApp {
    fn handle(&self, _request: Request<Body>) -> Response<Body> {
        unsafe {
            ::std::mem::zeroed()
        }
    }
}

static mut APP: &'static AppHandle = &DefaultApp;

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

pub fn run(addr: &str, thread_size: usize, app: &'static App) -> Result<()> {

    unsafe {
        APP = app;
    }

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

	let new_svc = move || {

        let pool = thread_pool.clone();

        service_fn(move |req| -> BoxFut {
            let rep = pool.spawn_fn(move || {
                let response = unsafe { APP.handle(req) };
                Ok(response)
            });

            Box::new(rep)
        })
     };

    let server = Server::bind(&addr).serve(new_svc).map_err(|e| eprintln!("server error: {}", e));
    hyper::rt::run(server);

	Ok(())
}
