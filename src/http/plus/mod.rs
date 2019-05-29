use rand::distributions::Alphanumeric;
use rand::{self, Rng};

pub mod client;
pub mod server;

pub fn random_alphanumeric(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .collect()
}
