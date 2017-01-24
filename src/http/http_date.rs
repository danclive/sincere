use chrono::*;

pub struct HTTPDate {
    d: DateTime<UTC>
}

impl HTTPDate {
    pub fn new() -> HTTPDate {
        HTTPDate {d: UTC::now(),}
    }
}

impl ToString for HTTPDate {
    fn to_string(&self) -> String {
        self.d.format("%a, %e %b %Y %H:%M:%S GMT").to_string()
    }
}

