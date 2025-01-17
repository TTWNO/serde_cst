use core::ops::{AddAssign, MulAssign};
use core::str::FromStr;

use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;

use crate::error::{Error, Result};
use crate::Gender;
#[cfg(feature = "alloc")]
use crate::Header;

pub struct Deserializer<'de> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    input: &'de [u8],
    byteswapped: Option<bool>,
}

impl<'de> Deserializer<'de> {
    pub fn from_str(input: &'de str) -> Self {
        Self::from_bytes(input.as_bytes())
    }
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    pub fn from_bytes(input: &'de [u8]) -> Self {
        Deserializer {
            input,
            byteswapped: None,
        }
    }
}

const CST_FLITE_HEADER: &str = "CMU_FLITE_CG_VOXDATA-v2.0";
const CST_LITTLE_ENDIAN_BYTE_VALUE: usize = 1;

// SERDE IS NOT A PARSING LIBRARY. This impl block defines a few basic parsing
// functions from scratch. More complicated formats may wish to use a dedicated
// parsing library to help implement their Serde deserializer.
impl<'de> Deserializer<'de> {
    fn validate_header(&mut self) -> Result<()> {
        if self.byteswapped.is_some() {
            return Ok(());
        }
        if !self.input.starts_with(CST_FLITE_HEADER.as_bytes()) {
            return Err(Error::InvalidHeader);
        }
        self.input = &self.input[CST_FLITE_HEADER.as_bytes().len() + 1..];
        self.byteswapped = Some(self.get_size_of_next()? != CST_LITTLE_ENDIAN_BYTE_VALUE);
        Ok(())
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
    fn parse_bool_unchecked_header(&mut self) -> Result<bool> {
        let required_size = 1;
        let size = self.get_size_of_next()?;
        if size != required_size {
            return Err(Error::ExpectedSize(size, 1));
        }
        // must use +1 to get rid of null byte
        let b = self.input.get(0..required_size + 1).ok_or(Error::Eof)?[0] != 0;
        // account for null byte: 2 instead of 1
        self.input = &self.input[2..];
        Ok(b)
    }
    fn parse_bool(&mut self) -> Result<bool> {
        self.validate_header()?;
        self.parse_bool_unchecked_header()
    }
    fn parse_str(&mut self) -> Result<&'de str> {
        self.validate_header()?;
        let size = self.get_size_of_next()?;
        #[cfg(feature = "debug")]
        println!("SIZE: {:?}", size);
        #[cfg(feature = "debug")]
        println!("BUFs: {:x?}", &self.input[..size]);
        let bytes = &self.input.get(0..size).ok_or(Error::Eof)?;
        if bytes[size - 1] != 0 {
            return Err(Error::WrongLength(size));
        }
        let s = core::str::from_utf8(&bytes[..size - 1])?;
        self.input = &self.input[size..];
        Ok(s)
    }
    fn read_bytes<const N: usize, const M: usize>(&mut self) -> Result<[u8; M]> {
        assert!(N >= M, "N must be greater than or equal to M");
        #[cfg(feature = "debug")]
        println!("BUF: {:x?}", &self.input[..N]);
        let n: &[u8; N] = self.input.get(..N).ok_or(Error::Eof)?.try_into().unwrap();
        let m: [u8; M] = n[..M].try_into().unwrap();
        self.input = &self.input[N..];
        Ok(m)
    }
    fn parse_digits(&mut self) -> Result<Vec<u8>> {
        let digit_chars: [u8; 10] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];
        let digits: Vec<u8> = self
            .input
            .iter()
            .take_while(|c| digit_chars.contains(c))
            .copied()
            .collect();
        self.input = &self.input[digits.len()..];
        Ok(digits)
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
    /*
    if !deserializer.input.is_empty() {
       return Err(Error::TrailingBytes);
    }
    */
    Ok(t)
}

struct StructValues<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    fields: &'static [&'static str],
    idx: usize,
}
impl<'a, 'de> StructValues<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, fields: &'static [&'static str]) -> Self {
        StructValues { de, fields, idx: 0 }
    }
}


// NOTE: array values do not work like this, they are loaded in one chunk
struct SeqValues<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: Option<usize>,
    idx: usize,
}
impl<'a, 'de> SeqValues<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        SeqValues {
            de,
            len: None,
            idx: 0,
        }
    }
    fn new_with_length(de: &'a mut Deserializer<'de>, len: usize) -> Self {
        SeqValues {
            de,
            len: Some(len),
            idx: 0,
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
        #[cfg(feature = "debug")]
        println!("BUFnes: {:?}", &self.de.input[..8]);
        #[cfg(feature = "debug")]
        println!("size-pre: {:?}", self.len);
        if self.len == None {
            let size = (&mut *self.de).get_size_of_next()?;
            self.len = Some(size);
        }
        #[cfg(feature = "debug")]
        println!("size-post: {:?}", self.len);
        // SAFETY: is checked above
        if self.len.unwrap() == self.idx {
            return Ok(None);
        }
        self.idx += 1;
        #[cfg(feature = "debug")]
        println!("idx: {}", self.idx);
        seed.deserialize(&mut *self.de).map(Some)
    }
}

// `SeqAccess` is provided to the `Visitor` to give it the ability to iterate
// through elements of the sequence.
impl<'de, 'a> MapAccess<'de> for StructValues<'a, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.fields.len() == self.idx {
            return Ok(None);
        }
        let field = seed.deserialize(&mut *self.de)?;
        self.idx += 1;
        Ok(Some(field))
    }
    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.de)
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
        #[cfg(feature = "debug")]
        println!("BUFks: {:x?}", &self.de.input[..8]);
        #[cfg(feature = "debug")]
        println!("TYPE: {}", std::any::type_name::<K>());
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
        #[cfg(feature = "debug")]
        println!("BUFvs: {:x?}", &self.de.input[..8]);
        #[cfg(feature = "debug")]
        println!("TYPE: {}", std::any::type_name::<V>());
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
        #[cfg(feature = "debug")]
        println!("BUFa: {:x?}", &self.input[..8]);
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
    // but in other cases Deserializers somechronos perform non-obvious mappings.
    // For example the TOML format has a Datechrono type and Serde's data model
    // does not. In the `toml` crate, a Datechrono in the input is deserialized by
    // mapping it to a Serde data model "struct" type with a special name and a
    // single field containing the Datechrono represented as a string.
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
        let val = i32::from_le_bytes(self.read_bytes::<4, 4>()?);
        visitor.visit_i32(val)
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
        let val = u8::from_le_bytes(self.read_bytes::<4, 1>()?);
        visitor.visit_u8(val)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = u16::from_le_bytes(self.read_bytes::<4, 2>()?);
        visitor.visit_u16(val)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = u32::from_le_bytes(self.read_bytes::<4, 4>()?);
        visitor.visit_u32(val)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("u64")
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        todo!("u128")
    }

    // Float parsing is stupidly hard.
    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let val = f32::from_le_bytes(self.read_bytes::<4, 4>()?);
        visitor.visit_f32(val)
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

    // Refer to the "Understanding deserializer lifechronos" page for information
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
        visitor.visit_unit()
    }

    // Unit struct means a named value containing no data.
    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    // As is done here, serializers are encouraged to treat newtype structs as
    // insignificant wrappers around the data they contain. That means not
    // parsing anything other than the contained value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
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
        self.validate_header()?;
        #[cfg(feature = "debug")]
        println!("SeqBUF: {:?}", &self.input[..8]);
        visitor.visit_seq(SeqValues::new(self))
    }

    // Tuples look just like sequences in JSON. Some formats may be able to
    // represent tuples more efficiently.
    //
    // As indicated by the length parameter, the `Deserialize` implementation
    // for a tuple in the Serde data model is required to know the length of the
    // tuple before even looking at the input data.
    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "debug")]
        println!("TUPLE SIZE: {}", len);
        visitor.visit_seq(SeqValues::new_with_length(self, len))
    }

    // Tuple structs look just like sequences in JSON.
    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "debug")]
        println!("TUPLE STRUCT SIZE: {}", len);
        self.deserialize_tuple(len, visitor)
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
    // the fields cannot be known ahead of chrono is probably a map.
    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "debug")]
        println!("FLs: {:?} ({})", fields, name);
        visitor.visit_map(StructValues::new(self, fields))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        #[cfg(feature = "debug")]
        println!("FVs: {:?}", variants);
        visitor.visit_enum(self.parse_str()?.into_deserializer())
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
        #[cfg(feature = "debug")]
        println!("BUFia: {:x?}", &self.input[..8]);
        self.deserialize_any(visitor)
    }
}

#[cfg(feature = "alloc")]
#[test]
fn test_vec() {
    extern crate alloc;
    use alloc::{vec, vec::Vec};
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x02\0\0\0\x05\0\0\0lang\0\x04\0\0\0eng\0";
    let expected: Vec<&str> = vec!["lang", "eng"];
    assert_eq!(expected, from_bytes::<Vec<&str>>(data.as_bytes()).unwrap());
}

#[cfg(feature = "alloc")]
#[test]
fn test_map() {
    extern crate alloc;
    use alloc::collections::BTreeMap;
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x05\0\0\0lang\0\x04\0\0\0eng\0";
    let mut expected: BTreeMap<&str, &str> = BTreeMap::new();
    expected.insert("lang", "eng");
    assert_eq!(
        expected,
        from_bytes::<BTreeMap<&str, &str>>(data.as_bytes()).unwrap()
    );
}

#[cfg(feature = "alloc")]
#[test]
fn test_struct() {
    use serde_with::{serde_as, DisplayFromStr};
    #[serde_as]
    #[derive(Deserialize, Debug, PartialEq)]
    struct Header {
        language: String,
        country: String,
        variant: String,
        #[serde_as(as = "DisplayFromStr")]
        age: u32,
        gender: Gender,
    }
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x09\0\0\0language\0\x04\0\0\0eng\0\x08\0\0\0country\0\x04\0\0\0USA\0\x08\0\0\0variant\0\x05\0\0\0none\0\x04\0\0\0age\0\x03\0\0\030\0\x07\0\0\0gender\0\x08\0\0\0unknown\0";
    let expected = Header {
        language: "eng".to_string(),
        country: "USA".to_string(),
        variant: "none".to_string(),
        age: 30,
        gender: Gender::Unknown,
    };
    assert_eq!(expected, from_bytes::<Header>(data.as_bytes()).unwrap());
}

#[test]
fn test_tuple() {
    let data =
        "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x01\0\0\0\x01\0\x05\0\0\0lang\0\x04\0\0\0eng\0";
    let expected = (true, "lang", "eng");
    assert_eq!(
        expected,
        from_bytes::<(bool, &str, &str)>(data.as_bytes()).unwrap()
    );
}

#[test]
fn test_bool() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x01\0\0\0\x09\0";
    let data2 = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x01\0\0\0\x00\0";
    assert_eq!(true, from_bytes(data.as_bytes()).unwrap());
    assert_eq!(false, from_bytes(data2.as_bytes()).unwrap());
}

#[test]
fn test_str() {
    let data = "CMU_FLITE_CG_VOXDATA-v2.0\0\x01\0\0\0\x09\0\0\0language\0";
    let expected: &str = "language";
    assert_eq!(expected, from_bytes::<&str>(data.as_bytes()).unwrap());
}

#[cfg(feature = "alloc")]
#[test]
fn test_file() {
    use crate::{EndOfFeatures, Features, Language};
    use chrono::NaiveDateTime;
    let data = include_bytes!("../data/cmu_us_slt.flitevox");
    let expected = Header {
        features: Features {
            language: "eng".to_string(),
            country: "USA".to_string(),
            variant: "none".to_string(),
            age: 30,
            gender: Gender::Unknown,
            build_date: chrono::NaiveDateTime::new(
                chrono::NaiveDate::from_ymd_opt(2017, 9, 14).unwrap(),
                chrono::NaiveTime::from_hms_opt(23, 37, 0).unwrap(),
            ),
            description: "unknown".to_string(),
            eng_shared: 0,
            copyright: "unknown".to_string(),
            num_dur_models: 3,
            num_param_models: 3,
            model_shape: 3,
            num_f0_models: 3,
            end_of_features: EndOfFeatures::EndOfFeatures,
        },
        name: "cmu_us_slt".to_string(),
    };
    assert_eq!(expected, from_bytes::<Header>(data).unwrap());
}
