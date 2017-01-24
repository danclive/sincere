use std::str::FromStr;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub enum Method {
    Get,
    Head,
    Post,
    Put,
    Delete,
    Connent,
    Options,
    Trace,
    Patch,
    NonStandard(String),
}

impl Method {
    pub fn as_str(&self) -> &str {
        match *self {
            Method::Get => "GET",
            Method::Head => "HEAD",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Connent => "CONNENT",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Patch => "PATCH",
            Method::NonStandard(ref m) => m,
        }
    }
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Method, ()> {
        Ok(match s {
            s if s == "GET" => Method::Get,
            s if s == "HEAD" => Method::Head,
            s if s == "POST" => Method::Post,
            s if s == "PUT" => Method::Put,
            s if s == "DELETE" => Method::Delete,
            s if s == "CONNENT" => Method::Connent,
            s if s == "OPTIONS" => Method::Options,
            s if s == "TRACE" => Method::Trace,
            s if s == "PATCH" => Method::Patch,
            s => {
                Method::NonStandard(s.to_owned())
            }
        })
    }
}

impl Display for Method {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), fmt::Error> {
        write!(formatter, "{}", self.as_str())
    }
}

impl PartialEq for Method {
    fn eq(&self, other: &Method) -> bool {
        match (self, other) {
            (&Method::NonStandard(ref m1), &Method::NonStandard(ref m2)) => m1 == m2,
            (&Method::Get, &Method::Get) => true,
            (&Method::Head, &Method::Head) => true,
            (&Method::Post, &Method::Post) => true,
            (&Method::Put, &Method::Put) => true,
            (&Method::Delete, &Method::Delete) => true,
            (&Method::Connent, &Method::Connent) => true,
            (&Method::Options, &Method::Options) => true,
            (&Method::Trace, &Method::Trace) => true,
            (&Method::Patch, &Method::Patch) => true,
            _ => false,
        }
    }
}

impl Eq for Method {}
