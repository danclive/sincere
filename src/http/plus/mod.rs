use rand::{self, Rng};

pub mod server;
pub mod client;

pub use hyper::mime;

pub fn random_alphanumeric(len: usize) -> String {
    rand::thread_rng().gen_ascii_chars().take(len).collect()
}
