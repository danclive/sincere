use chrono::{DateTime, Utc};

pub struct HTTPDate {
    d: DateTime<Utc>
}

impl HTTPDate {
    pub fn new() -> HTTPDate {
        HTTPDate {d: Utc::now(),}
    }
}

impl ToString for HTTPDate {
    fn to_string(&self) -> String {
        self.d.format("%a, %e %b %Y %H:%M:%S GMT").to_string()
    }
}

