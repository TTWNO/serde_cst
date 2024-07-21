use crate::Header;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(from = "_Body", into = "_Body")]
pub struct Body {
    pub db_types: Vec<String>,
    pub num_types: i32,
    pub sample_rate: i32,
    pub f0_mean: f32,
    pub f0_stddev: f32,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct _Body(pub Vec<String>, pub i32, pub i32, pub f32, pub f32);

impl From<_Body> for Body {
    fn from(body: _Body) -> Body {
        Body {
            db_types: body.0,
            num_types: body.1,
            sample_rate: body.2,
            f0_mean: body.3,
            f0_stddev: body.4,
        }
    }
}

impl From<Body> for _Body {
    fn from(body: Body) -> _Body {
        _Body(
            body.db_types,
            body.num_types,
            body.sample_rate,
            body.f0_mean,
            body.f0_stddev,
        )
    }
}

#[test]
fn test_cluster_voice() {
    use crate::{de::from_bytes, EndOfFeatures, Features, Gender, Language};
    use chrono::NaiveDateTime;
    let data = include_bytes!("../data/cmu_us_slt.flitevox");
    let expected_head = Header {
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
    let expected = (
        expected_head,
        _Body(
            vec![
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
            0x7c,
            0x3e80,
            f32::from_le_bytes([0, 0, 0x2c, 0x43]),
            f32::from_le_bytes([0, 0, 0xd8, 0x41]),
        ),
    );
    assert_eq!(expected, from_bytes::<(Header, _Body)>(data).unwrap());
}
