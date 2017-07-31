use std::collections::HashMap;
use http::Request;
use http::Response;

pub struct Context {
	pub request: Request,
	pub response: Response,
	pub contexts: HashMap<String, String>,
	stop: bool
}

impl Context {
	pub fn new(request: Request) -> Context {
		let response = Response::empty(200);

		Context {
			request: request,
			response: response,
			contexts: HashMap::new(),
			stop: false
		}
	}

	pub fn stop(&mut self) {
		self.stop = true;
	}

	pub fn next(&self) -> bool {
		!self.stop
	}
}

