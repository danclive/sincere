pub use serde::de;
pub use serde::de::value::Error;
pub use url::form_urlencoded::parse;

mod decode;

pub fn from_bytes<'de, T>(input: &'de [u8]) -> Result<T, Error>
    where T: de::Deserialize<'de>
{
    T::deserialize(decode::Decoder::new(parse(input)))
}

pub fn from_str<'de, T>(input: &'de str) -> Result<T, Error>
    where T: de::Deserialize<'de>
{
    from_bytes(input.as_bytes())
}
