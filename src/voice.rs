use crate::{error::Error, Header};
use serde::{Deserialize, Deserializer, de::DeserializeSeed, de::value::SeqDeserializer, Serialize, de::Visitor, de::SeqAccess, de};
use serde_dis::{DeserializeWithDiscriminant};
use core::{fmt, marker::PhantomData};

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum CstVal {
    // no idea what this means
    Cons(i32) = 0,
    Int(i32) = 1,
    Float(f32) = 3,
    Str(String) = 5,
    FirstFree(i32) = 7,
    Other(i32) = 54
}
struct CstValVisitor;
impl<'de> Visitor<'de> for CstValVisitor {
    type Value = CstVal;
    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("A CstVal consisting of a singe byte, which determintes the type that follows")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> 
    where A: SeqAccess<'de> {
        let discrim = seq.next_element()?
                    .ok_or(de::Error::invalid_length(0, &self))?;
        println!("CstValue discriminant: {}", discrim);
        match discrim {
            0 => {
                let v = seq.next_element()?
                    .ok_or(de::Error::invalid_length(1, &self))?;
                Ok(CstVal::Cons(v))
            },
            1 => {
                let v = seq.next_element()?
                    .ok_or(de::Error::invalid_length(1, &self))?;
                Ok(CstVal::Int(v))
            },
            3 => {
                let v = seq.next_element()?
                    .ok_or(de::Error::invalid_length(1, &self))?;
                Ok(CstVal::Float(v))
            },
            5 => {
                let v = seq.next_element()?
                    .ok_or(de::Error::invalid_length(1, &self))?;
                Ok(CstVal::Str(v))
            },
            7 => {
                let v = seq.next_element()?
                    .ok_or(de::Error::invalid_length(1, &self))?;
                Ok(CstVal::FirstFree(v))
            },
            _ => {
                let v = seq.next_element()?
                    .ok_or(de::Error::invalid_length(1, &self))?;
                Ok(CstVal::Other(v))
            },
        }
    }
}
impl<'de> Deserialize<'de> for CstVal {
    fn deserialize<D>(deser: D) -> Result<Self, D::Error> 
    where D: Deserializer<'de> {
        deser.deserialize_seq(CstValVisitor)
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct TreeNode (
    u8, // feat
    u8, // op
    u16, // no of tree
    CstVal, // value expession
);

#[derive(Deserialize, Debug, PartialEq)]
pub struct TreeFeatures(Vec<String>);

#[derive(Deserialize, Debug, PartialEq)]
pub struct Tree (
    TreeNode,
    TreeFeatures,
);

#[derive(Deserialize, Debug, PartialEq)]
pub struct F0Tree(Vec<Tree>);

struct FixedSeqValuesVisitor<'de, D> {
    len: usize,
    idx: usize,
    _marker: &'de core::marker::PhantomData<D>,
}
impl<'de, D> FixedSeqValuesVisitor<'de, D> {
    fn new(len: usize) -> Self {
        FixedSeqValuesVisitor {
            len,
            idx: 0,
            _marker: &core::marker::PhantomData,
        }
    }
}
impl<'de, D> Visitor<'de> for FixedSeqValuesVisitor<'de, D>
where D: Deserialize<'de> {
    type Value = Vec<D>;
    fn expecting(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        fmt.write_fmt(format_args!("A fixed vector of length {}", self.len))
    }
    fn visit_seq<A>(self, mut seq: A) -> core::result::Result<Self::Value, A::Error>
    where A: SeqAccess<'de> {
        let mut vec = Vec::with_capacity(self.len);
        for i in 0..self.len {
            let val = seq.next_element()?
                .ok_or(de::Error::invalid_length(i, &self))?;
            vec.push(val);
        }
        Ok(vec)
    }
}

#[derive(Debug, PartialEq)]
pub struct TreeDb {
    header: Header,
    body: Body,
}
struct TreeDbVisitor;
impl<'de> Visitor<'de> for TreeDbVisitor {
    type Value = TreeDb;
    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("A tree datebase which begins with a header and ends with a body")
    }
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> 
    where A: SeqAccess<'de> {
        let header = seq.next_element()?
                .ok_or(de::Error::invalid_length(0, &self))?;
        let body_deserial = BodyDeserializer { header: &header };
        let body = seq.next_element_seed(body_deserial)?
                .ok_or(de::Error::invalid_length(1, &self))?;
        Ok(TreeDb { header, body })
    }
}
impl<'de> Deserialize<'de> for TreeDb {
    fn deserialize<D>(deserializer: D) -> Result<TreeDb, D::Error> 
    where D: Deserializer<'de> {
        deserializer.deserialize_tuple(2, TreeDbVisitor)
    }
}

#[derive(Debug, PartialEq)]
pub struct Body {
    pub db_types: Vec<String>,
    pub num_types: i32,
    pub sample_rate: i32,
    pub f0_mean: f32,
    pub f0_stddev: f32,
    pub f0_trees: Vec<F0Tree>,
}

struct FixedLengthSeq<T> {
    pub len: usize,
    pub _marker: PhantomData<T>,
}
impl<T> FixedLengthSeq<T> {
    fn from_len(len: usize) -> Self {
        FixedLengthSeq { len, _marker: PhantomData }
    }
}
impl<'de, T> DeserializeSeed<'de> for FixedLengthSeq<T>
where T: Deserialize<'de> + 'de, {
    type Value = Vec<T>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> 
    where D: Deserializer<'de> {
        deserializer.deserialize_seq(FixedSeqValuesVisitor::new(self.len))
    }
}

struct BodyVisitor<'a> {
    header: &'a Header,
}
impl<'a> BodyVisitor<'a> {
    fn new(header: &'a Header) -> Self {
        BodyVisitor { header } 
    }
}
impl<'a, 'de> Visitor<'de> for BodyVisitor<'a> {
    type Value = Body;
    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str("A body of a Festivel CG (cluster gen) voice")
    }
    fn visit_seq<V>(self, mut seq: V) -> Result<Body, V::Error> 
    where V: SeqAccess<'de> {
        Ok(Body {
            db_types: seq.next_element()?
                .ok_or(de::Error::invalid_length(0, &self))?,
            num_types: seq.next_element()?
                .ok_or(de::Error::invalid_length(1, &self))?,
            sample_rate: seq.next_element()?
                .ok_or(de::Error::invalid_length(2, &self))?,
            f0_mean: seq.next_element()?
                .ok_or(de::Error::invalid_length(3, &self))?,
            f0_stddev: seq.next_element()?
                .ok_or(de::Error::invalid_length(4, &self))?,
            f0_trees: seq.next_element_seed(FixedLengthSeq::from_len(self.header.features.num_f0_models.try_into().unwrap()))?
                .ok_or(de::Error::invalid_length(5, &self))?,
        })
    }
}

struct BodyDeserializer<'a> {
    header: &'a Header,
}

impl<'de, 'a> DeserializeSeed<'de> for BodyDeserializer<'a> {
    type Value = Body;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error> 
    where D: Deserializer<'de> {
        deserializer.deserialize_tuple(6, BodyVisitor::new(self.header))
    }
}

/*
struct BodyDeserializer;
impl<'de> Visitor<'de> for BodyVisitor {
    type Error = Error;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A body attached to a header.");
    }
    fn visit_seq<V>() -> Result<Self::Value, V::Error> 
    where V: SeqAccess<'de> {
        let header = seq.next_element()?;
        let body = seq.next_element_seed(BodySeed { header })?; 
        Ok(body)
    }
}
*/

#[test]
fn test_cluster_voice() {
    use crate::{de::from_bytes, EndOfFeatures, Features, Gender, Language};
    use chrono::NaiveDateTime;
    let data = include_bytes!("../data/cmu_us_slt.flitevox");
    let header = Header {
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
    let body = Body {
            db_types: vec![
                "aa_1".to_string(),
                "aa_2".to_string(),
                "aa_3".to_string(),
                "ae_1".to_string(),
                "ae_2".to_string(),
                "ae_3".to_string(),
                "ah_1".to_string(),
                "ah_2".to_string(),
                "ah_3".to_string(),
                "ao_1".to_string(),
                "ao_2".to_string(),
                "ao_3".to_string(),
                "aw_1".to_string(),
                "aw_2".to_string(),
                "aw_3".to_string(),
                "ax_1".to_string(),
                "ax_2".to_string(),
                "ax_3".to_string(),
                "ay_1".to_string(),
                "ay_2".to_string(),
                "ay_3".to_string(),
                "b_1".to_string(),
                "b_2".to_string(),
                "b_3".to_string(),
                "ch_1".to_string(),
                "ch_2".to_string(),
                "ch_3".to_string(),
                "d_1".to_string(),
                "d_2".to_string(),
                "d_3".to_string(),
                "dh_1".to_string(),
                "dh_2".to_string(),
                "dh_3".to_string(),
                "eh_1".to_string(),
                "eh_2".to_string(),
                "eh_3".to_string(),
                "er_1".to_string(),
                "er_2".to_string(),
                "er_3".to_string(),
                "ey_1".to_string(),
                "ey_2".to_string(),
                "ey_3".to_string(),
                "f_1".to_string(),
                "f_2".to_string(),
                "f_3".to_string(),
                "g_1".to_string(),
                "g_2".to_string(),
                "g_3".to_string(),
                "hh_1".to_string(),
                "hh_2".to_string(),
                "hh_3".to_string(),
                "ih_1".to_string(),
                "ih_2".to_string(),
                "ih_3".to_string(),
                "iy_1".to_string(),
                "iy_2".to_string(),
                "iy_3".to_string(),
                "jh_1".to_string(),
                "jh_2".to_string(),
                "jh_3".to_string(),
                "k_1".to_string(),
                "k_2".to_string(),
                "k_3".to_string(),
                "l_1".to_string(),
                "l_2".to_string(),
                "l_3".to_string(),
                "m_1".to_string(),
                "m_2".to_string(),
                "m_3".to_string(),
                "n_1".to_string(),
                "n_2".to_string(),
                "n_3".to_string(),
                "ng_1".to_string(),
                "ng_2".to_string(),
                "ng_3".to_string(),
                "ow_1".to_string(),
                "ow_2".to_string(),
                "ow_3".to_string(),
                "oy_1".to_string(),
                "oy_2".to_string(),
                "oy_3".to_string(),
                "p_1".to_string(),
                "p_2".to_string(),
                "p_3".to_string(),
                "pau_1".to_string(),
                "pau_2".to_string(),
                "pau_3".to_string(),
                "pau_5".to_string(),
                "r_1".to_string(),
                "r_2".to_string(),
                "r_3".to_string(),
                "s_1".to_string(),
                "s_2".to_string(),
                "s_3".to_string(),
                "sh_1".to_string(),
                "sh_2".to_string(),
                "sh_3".to_string(),
                "t_1".to_string(),
                "t_2".to_string(),
                "t_3".to_string(),
                "th_1".to_string(),
                "th_2".to_string(),
                "th_3".to_string(),
                "uh_1".to_string(),
                "uh_2".to_string(),
                "uh_3".to_string(),
                "uw_1".to_string(),
                "uw_2".to_string(),
                "uw_3".to_string(),
                "v_1".to_string(),
                "v_2".to_string(),
                "v_3".to_string(),
                "w_1".to_string(),
                "w_2".to_string(),
                "w_3".to_string(),
                "y_1".to_string(),
                "y_2".to_string(),
                "y_3".to_string(),
                "z_1".to_string(),
                "z_2".to_string(),
                "z_3".to_string(),
                "zh_1".to_string(),
                "zh_2".to_string(),
                "zh_3".to_string(),
            ],
            num_types: 0x7c,
            sample_rate: 0x3e80,
            f0_mean: f32::from_le_bytes([0, 0, 0x2c, 0x43]),
            f0_stddev: f32::from_le_bytes([0, 0, 0xd8, 0x41]),
            f0_trees: vec![]
        };
    let expected = TreeDb {
        header, body
    };
    assert_eq!(expected, from_bytes::<TreeDb>(data).unwrap());
}
