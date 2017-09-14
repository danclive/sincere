use std::borrow::Cow;

use serde::de::{self, IntoDeserializer};
use serde::de::value::MapDeserializer;
use url::form_urlencoded::Parse;

pub use serde::de::value::Error;


pub struct Decoder<'de> {
    inner: MapDeserializer<'de, PartIterator<'de>, Error>
}

impl<'de> Decoder<'de> {
    pub fn new(parser: Parse<'de>) -> Self {
        Decoder {
            inner: MapDeserializer::new(PartIterator(parser))
        }
    }
}

impl<'de> de::Deserializer<'de> for Decoder<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor<'de>
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor<'de>
    {
        visitor.visit_map(self.inner)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor<'de>
    {
        visitor.visit_seq(self.inner)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V:  de::Visitor<'de>
    {
        self.inner.end()?;
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool
        u8
        u16
        u32
        u64
        i8
        i16
        i32
        i64
        f32
        f64
        char
        str
        string
        option
        bytes
        byte_buf
        unit_struct
        newtype_struct
        tuple_struct
        struct
        identifier
        tuple
        enum
        ignored_any
    }
}

struct PartIterator<'de>(Parse<'de>);

impl<'de> Iterator for PartIterator<'de> {
    type Item = (Part<'de>, Part<'de>);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (Part(k), Part(v)))
    }
}

struct Part<'de>(Cow<'de, str>);

impl<'de> IntoDeserializer<'de> for Part<'de> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

macro_rules! forward_parsed_value {
    ($($ty:ident => $method:ident,)*) => {
        $(
            fn $method<V>(self, visitor: V) -> Result<V::Value, Self::Error>
                where V: de::Visitor<'de>
            {
                match self.0.parse::<$ty>() {
                    Ok(val) => val.into_deserializer().$method(visitor),
                    Err(e) => Err(de::Error::custom(e))
                }
            }
        )*
    }
}

impl<'de> de::Deserializer<'de> for Part<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor<'de>
    {
        self.0.into_deserializer().deserialize_any(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor<'de>
    {
        visitor.visit_some(self)
    }

    forward_to_deserialize_any! {
        char
        str
        string
        unit
        bytes
        byte_buf
        unit_struct
        newtype_struct
        tuple_struct
        struct
        identifier
        tuple
        enum
        ignored_any
        seq
        map
    }

    forward_parsed_value! {
        bool => deserialize_bool,
        u8 => deserialize_u8,
        u16 => deserialize_u16,
        u32 => deserialize_u32,
        u64 => deserialize_u64,
        i8 => deserialize_i8,
        i16 => deserialize_i16,
        i32 => deserialize_i32,
        i64 => deserialize_i64,
        f32 => deserialize_f32,
        f64 => deserialize_f64,
    }
}
