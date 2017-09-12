use std::fs;
use std::io::BufReader;
use std::sync::Arc;

use rustls;

pub struct TlsConfig<'a> {
    cert_path: &'a str,
    privkey_path: &'a str,
}

impl<'a> TlsConfig<'a> {
    pub fn new(cert_path: &'a str, privkey_path: &'a str) -> TlsConfig<'a> {
        TlsConfig {
            cert_path: cert_path,
            privkey_path: privkey_path
        }
    }

    fn load_certs(&self) -> Vec<rustls::Certificate> {
        let certfile = fs::File::open(self.cert_path).expect("cannot open certificate file");
        let mut reader = BufReader::new(certfile);

        rustls::internal::pemfile::certs(&mut reader).expect("cannot open certificate file")
    }

    fn load_private_key(&self) -> rustls::PrivateKey {
        let keyfile = fs::File::open(self.privkey_path).expect("cannot open private key file");
        let mut reader = BufReader::new(keyfile);
        let keys = rustls::internal::pemfile::rsa_private_keys(&mut reader).expect("cannot open private key file");

        if keys.len() != 1 {
            panic!("{:?}", "cannot open private key file");
        }

        keys[0].clone()
    }

    pub fn make_config(&self) -> Arc<rustls::ServerConfig> {
        let mut config = rustls::ServerConfig::new();
        config.set_single_cert(self.load_certs(), self.load_private_key());

        let session_memory_cache = rustls::ServerSessionMemoryCache::new(1024);
        config.set_persistence(session_memory_cache);

        Arc::new(config)
    }
}
