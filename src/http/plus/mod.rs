use rand::{self, Rng};
use rand::distributions::Alphanumeric;

pub mod server;
pub mod client;

pub fn random_alphanumeric(len: usize) -> String {
    rand::thread_rng().sample_iter(&Alphanumeric).take(len).collect()
}
