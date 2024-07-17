use core::ops::{AddAssign, MulAssign, Neg};
use core::str::FromStr;

use serde::Deserialize;
use serde::de::{
    self, DeserializeSeed, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};

use crate::error::{Error, Result};

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de [u8],
    has_header: bool,
}

impl<'de> Deserializer<'de> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer { input, has_header: false }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
/*
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}
*/

const CST_FLITE_HEADER: &str = "CMU_FLITE_CG_VOXDATA-v2.0";

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    fn validate_header(&mut self) -> Result<()> {
        if self.has_header {
            return Ok(());
        }
        if !self.input.starts_with(CST_FLITE_HEADER.as_bytes()) {
            return Err(Error::InvalidHeader);
        }
        self.has_header = true;
        self.input = &self.input[CST_FLITE_HEADER.as_bytes().len()+1..];
        Ok(())
    }
    // Look at the first character in the input without consuming it.
    fn peek_byte(&mut self) -> Result<u8> {
        self.input.iter().next().ok_or(Error::Eof).copied()
    }

    // Consume the first character in the input.
    fn next_byte(&mut self) -> Result<u8> {
        let ch = self.peek_byte()?;
        self.input = &self.input[1..];
        Ok(ch)
    }
    fn get_size_of_next(&mut self) -> Result<usize> {
        let bytes = self.input.get(0..4).ok_or(Error::Eof)?;
        #[cfg(target_pointer_width = "64")]
        let result = usize::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], 0, 0, 0, 0]);
        #[cfg(target_pointer_width = "32")]
        let result = usize::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        #[cfg(target_pointer_width = "16")]
        compile_error!("This crate is not compatible with 16-bit architectures.");
        self.input = &self.input[4..];
        Ok(result)
    }
    fn parse_bool(&mut self) -> Result<bool> {
        let required_size = 1;
        self.validate_header()?;
        let size = self.get_size_of_next()?;
        if size != required_size {
            return Err(Error::ExpectedSize(size, 1));
        }
        // must use +1 to get rid of null byte
        let b = self.input.get(0..required_size+1).ok_or(Error::Eof)?[0] != 0;
        // account for null byte: 2 instead of 1
        self.input = &self.input[2..];
        Ok(b)
    }
    fn parse_str(&mut self) -> Result<&'de str> {
        self.validate_header()?;
        println!("BUF: {:?}", self.input);
        let size = self.get_size_of_next()?;
        let bytes = &self.input.get(0..size).ok_or(Error::Eof)?;
        if bytes[size-1] != 0 {
            return Err(Error::WrongLength(size));
        }
        let s = core::str::from_utf8(&bytes[..size-1])?;
        self.input = &self.input[size..];
        println!("\tAFTER: {:?}", self.input);
        Ok(s)
    }
    fn parse_unsigned<T>(&mut self) -> Result<T>
    where T: AddAssign<T> + MulAssign<T> + From<u8> + FromStr,
        Error: From<<T as FromStr>::Err>
    {
        let ascii = self.parse_str()?;
        Ok(T::from_str(ascii)?)
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if !deserializer.input.is_empty() {
       return Err(Error::TrailingBytes);
    }
    Ok(t)
}

struct Enum<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> Enum<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Enum { de }
    }
}

// `EnumAccess` is provided to the `Visitor` to give it the ability to determine
// which variant of the enum is supposed to be deserialized.
//
// Note that all enum deserialization methods in Serde refer exclusively to the
// "externally tagged" enum representation.
/*
impl<'de, 'a> EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        // The `deserialize_enum` method parsed a `{` character so we are
        // currently inside of a map. The seed will be deserializing itself from
        // the key of the map.
        let val = seed.deserialize(&mut *self.de)?;
        // Parse the colon separating map key from value.
        if self.de.next_char()? == ':' {
            Ok((val, self))
        } else {
            Err(Error::ExpectedMapColon)
        }
    }
}
*/

// `VariantAccess` is provided to the `Visitor` to give it the ability to see
// the content of the single variant that it decided to deserialize.
/*
impl<'de, 'a> VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    // If the `Visitor` expected this variant to be a unit variant, the input
    // should have been the plain string case handled in `deserialize_enum`.
    fn unit_variant(self) -> Result<()> {
        Err(Error::ExpectedString)
    }

    // Newtype variants are represented in JSON as `{ NAME: VALUE }` so
    // deserialize the value here.
    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    // Tuple variants are represented in JSON as `{ NAME: [DATA...] }` so
    // deserialize the sequence of data here.
    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self.de, visitor)
    }

    // Struct variants are represented in JSON as `{ NAME: { K: V, ... } }` so
    // deserialize the inner map here.
    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self.de, visitor)
    }
}
*/


struct SeqValues<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>
}
impl<'a, 'de> SeqValues<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        SeqValues {
            de,
        }
    }
}
// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> SeqAccess<'de> for SeqValues<'a, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// `MapAccess` is provided to the `Visitor` to give it the ability to iterate
// through entries of the map.
impl<'de, 'a> MapAccess<'de> for SeqValues<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.input.is_empty() {
            return Ok(None);
        }
        // Deserialize a map key.
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        // Deserialize a map value.
        seed.deserialize(&mut *self.de)
    }
}


impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    // Look at the input data to decide what Serde data model type to
    // deserialize as. Not all data formats are able to support this operation.
    // Formats that support `deserialize_any` are known as self-describing.
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        /*
        match self.peek_char()? {
            'n' => self.deserialize_unit(visitor),
            't' | 'f' => self.deserialize_bool(visitor),
            '"' => self.deserialize_str(visitor),
            '0'..='9' => self.deserialize_u64(visitor),
            '-' => self.deserialize_i64(visitor),
            '[' => self.deserialize_seq(visitor),
            '{' => self.deserialize_map(visitor),
            _ => Err(Error::Syntax),
        }
        */
        todo!("any")
    }

    // Uses the `parse_bool` parsing function defined above to read the JSON
    // identifier `true` or `false` from the input.
    //
    // Parsing refers to looking at the input and deciding that it contains the
    // JSON value `true` or `false`.
    //
    // Deserialization refers to mapping that JSON value into Serde's data
    // model by invoking one of the `Visitor` methods. In the case of JSON and
    // bool that mapping is straightforward so the distinction may seem silly,
    // but in other cases Deserializers sometimes perform non-obvious mappings.
    // For example the TOML format has a Datetime type and Serde's data model
    // does not. In the `toml` crate, a Datetime in the input is deserialized by
    // mapping it to a Serde data model "struct" type with a special name and a
    // single field containing the Datetime represented as a string.
    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    // The `parse_signed` function is generic over the integer type `T` so here
    // it is invoked with `T=i8`. The next 8 methods are similar.
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("i8")
        //visitor.visit_i8(self.parse_signed()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("i16")
        //visitor.visit_i16(self.parse_signed()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("i32")
        //visitor.visit_i32(self.parse_signed()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("i64")
        //visitor.visit_i64(self.parse_signed()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_unsigned()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_unsigned()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_unsigned()?)
    
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_unsigned()?)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u128(self.parse_unsigned()?)
    }

    // Float parsing is stupidly hard.
    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("f32")
    }

    // Float parsing is stupidly hard.
    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("f64")
    }

    // The `Serializer` implementation on the previous page serialized chars as
    // single-character strings so handle that representation here.
    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Parse a string, check that it is one character, call `visit_char`.
        todo!("char")
    }

    // Refer to the "Understanding deserializer lifetimes" page for information
    // about the three deserialization flavors of strings in Serde.
    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.parse_str()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // The `Serializer` implementation on the previous page serialized byte
    // arrays as JSON arrays of bytes. Handle that representation here.
    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("bytes")
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("bytebuf")
    }

    // An absent optional is represented as the JSON `null` and a present
    // optional is represented as just the contained value.
    //
    // As commented in `Serializer` implementation, this is a lossy
    // representation. For example the values `Some(())` and `None` both
    // serialize as just `null`. Unfortunately this is typically what people
    // expect when working with JSON. Other formats are encouraged to behave
    // more intelligently if possible.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("option")
    }

    // In Serde, unit means an anonymous value containing no data.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("unit")
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    // Deserialization of compound types like sequences and maps happens by
    // passing the visitor an "Access" object that gives it the ability to
    // iterate through the data contained in the sequence.
    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(SeqValues::new(self))
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("tuple struct")
    }

    // Much like `deserialize_seq` but calls the visitors `visit_map` method
    // with a `MapAccess` implementation, rather than the visitor's `visit_seq`
    // method with a `SeqAccess` implementation.
    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(SeqValues::new(self))
    }

    // Structs look just like maps in JSON.
    //
    // Notice the `fields` parameter - a "struct" in the Serde data model means
    // that the `Deserialize` implementation is required to know what the fields
    // are before even looking at the input data. Any key-value pairing in which
    // the fields cannot be known ahead of time is probably a map.
    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("enum")
    }

    // An identifier in Serde is the type that identifies a field of a struct or
    // the variant of an enum. In JSON, struct fields and enum variants are
    // represented as strings. In other formats they may be represented as
    // numeric indices.
    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    // Like `deserialize_any` but indicates to the `Deserializer` that it makes
    // no difference which `Visitor` method is called because the data is
    // ignored.
    //
    // Some deserializers are able to implement this more efficiently than
    // `deserialize_any`, for example by rapidly skipping over matched
    // delimiters without paying close attention to the data in between.
    //
    // Some formats are not able to implement this at all. Formats that can
    // implement `deserialize_any` and `deserialize_ignored_any` are known as
    // self-describing.
    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

#[test]
fn test_map() {
    extern crate alloc;
    use alloc::collections::BTreeMap;
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x05\0\0\0lang\0\x04\0\0\0eng\0";
    let mut expected: BTreeMap<&str, &str> = BTreeMap::new();
    expected.insert("lang", "eng");
    assert_eq!(expected, from_bytes::<BTreeMap<&str, &str>>(data.as_bytes()).unwrap());
}

#[derive(Deserialize, Debug, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    #[default]
    Unknown
}

#[test]
fn test_struct() {
#[derive(Deserialize, Debug, PartialEq)]
    struct HeaderParts {
        byteswap: bool,
        language: String,
        country: String,
        variant: String,
        age: u32,
        gender: Gender,
    }
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x02\0\0\0\0\0\x09\0\0\0language\0\x04\0\0\0eng\0\x08\0\0\0country\0\x04\0\0\0USA\0\x08\0\0\0variant\0\x05\0\0\0none\0\x04\0\0\0age\0\x03\0\0\030\0\x07\0\0\0gender\0\x08\0\0\0unknown\0";
    let expected = HeaderParts {
        byteswap: false,
        language: "English".to_string(),
        country: "USA".to_string(),
        variant: "none".to_string(),
        age: 30,
        gender: Gender::Unknown,
    };
    assert_eq!(expected, from_bytes::<HeaderParts>(data.as_bytes()).unwrap());
}

#[test]
fn test_tuple() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\0\0\x05\0\0\0lang\0\x04\0\0\0eng\0";
    let expected = (false, "lang", "eng");
    assert_eq!(expected, from_bytes::<(bool, &str, &str)>(data.as_bytes()).unwrap());
}

#[test]
fn test_bool() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x09\0";
    let data2 = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x00\0";
    assert_eq!(true, from_bytes(data.as_bytes()).unwrap());
    assert_eq!(false, from_bytes(data2.as_bytes()).unwrap());
}

#[test]
fn test_str() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x09\0\0\0language\0";
    let expected: &str = "language";
    assert_eq!(expected, from_bytes::<&str>(data.as_bytes()).unwrap());
}

#[test]
fn test_u8() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x04\0\0\0255\0";
    let expected: u8 = 255;
    assert_eq!(expected, from_bytes::<u8>(data.as_bytes()).unwrap());
}

#[test]
fn test_u16() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x06\0\0\065535\0";
    let expected = u16::MAX;
    assert_eq!(expected, from_bytes::<u16>(data.as_bytes()).unwrap());
}

#[test]
fn test_u32() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x0B\0\0\04294967295\0";
    let expected = u32::MAX;
    assert_eq!(expected, from_bytes::<u32>(data.as_bytes()).unwrap());
}

#[test]
fn test_u64() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x15\0\0\018446744073709551615\0";
    let expected = u64::MAX;
    assert_eq!(expected, from_bytes::<u64>(data.as_bytes()).unwrap());
}

#[test]
fn test_u128() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x28\0\0\0340282366920938463463374607431768211455\0";
    let expected = u128::MAX;
    assert_eq!(expected, from_bytes::<u128>(data.as_bytes()).unwrap());
}
