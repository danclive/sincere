//! Sincere is a micro web framework for Rust(stable) based on 
//! [hyper](https://github.com/hyperium/hyper) and multithreadind. Style like [koa](https://github.com/koajs/koa).
//! The same, which aims to be a smaller, more expressive, and more robust foundation for
//! web applications and APIs. Sincere does not bundle any middleware within core,
//! and provides an elegant suite of methods that make writing servers fast and enjoyable.
//!
//! ## Usage
//!
//! First, add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! sincere = "0.5.8"
//! ```
//!
//! Then, add this to your crate root:
//!
//! ```rust
//! extern crate sincere;
//! ```
//!
//! # Example
//! ```rust
//! use sincere::App;
//!
//! fn main() {
//!    let mut app = App::new();
//!
//!    app.get("/", |context| {
//!        context.response.from_text("Hello world!").unwrap();
//!    });
//!
//!    //app.run("127.0.0.1:8000", 20).unwrap();
//! }
//! ```
//!
//!


extern crate chrono;
extern crate regex;
#[macro_use]
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate httparse;
extern crate num_cpus;
extern crate libc;
extern crate twoway;
extern crate tempdir;
extern crate rand;

extern crate hyper;
extern crate futures;
extern crate futures_cpupool;

#[allow(unused_imports)]
#[macro_use]
extern crate queen_log;
extern crate mime;
extern crate mime_guess;
pub extern crate nson;

pub mod http;
pub mod app;
pub mod text;
pub mod util;
pub mod error;

pub use self::app::App;
pub use self::error::Error;

#[doc(hidden)]
pub use queen_log::*;

pub mod log {
    pub use queen_log::color;
    pub use queen_log::{LOG, MAX_LEVEL};
    pub use queen_log::{Logger, DefaultLogger};
    pub use queen_log::{Log, init, Level, Record, Metadata};
}
