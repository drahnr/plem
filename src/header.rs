use log::{error, info, trace, warn};

use std::collections::{HashMap, HashSet};

use lazy_static::*;

use std::cmp::{Eq, Ord, PartialEq, PartialOrd};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub(crate) struct HeaderInfo {
    pub repeat: usize,
    pub steps: usize,
    pub pallet_name: String,
    pub extrinsic: String,
}

pub type ColumnIndex = usize;
pub type ColumnName = String;

pub(crate) struct HeaderColumns {
    pub columns: HashMap<ColumnIndex, ColumnName>,
}

use nom::branch::alt;
use nom::bytes::complete::{take_till, take_until};
use nom::character::complete::alphanumeric1;
use nom::character::complete::space0;
use nom::character::complete::{char, digit1, line_ending, not_line_ending};
use nom::character::{is_alphanumeric, is_digit};
use nom::combinator::{all_consuming, complete, map, map_parser, not, opt};
use nom::multi::many1;
use nom::number::complete::be_u64;
use nom::sequence::{delimited, preceded};
use nom::sequence::{terminated, tuple};
use nom::IResult;

#[derive(Clone, Debug)]
enum Value {
    Numeric(u64),
    String(String),
}

impl Value {
    fn as_type(&self) -> ValueType {
        match self {
            Self::Numeric(_) => ValueType::Numeric,
            Self::String(_) => ValueType::String,
        }
    }
}

impl Value {
    pub fn as_str(&self) -> &str {
        if let Self::String(ref s) = self {
            return s.as_str();
        }
        unreachable!("must")
    }
    pub fn as_usize(&self) -> usize {
        if let Self::Numeric(ref n) = self {
            return *n as usize;
        }
        unreachable!("must")
    }
}

#[derive(Clone, Copy, Debug)]
enum ValueType {
    Numeric,
    String,
}

lazy_static! {
    static ref SUPPORTED: HashMap<&'static str, ValueType> = {
        let mut m = HashMap::with_capacity(4);
        m.insert("pallet", ValueType::String);
        m.insert("extrinsic", ValueType::String);
        m.insert("steps", ValueType::Numeric);
        m.insert("repeat", ValueType::Numeric);
        m
    };
}

fn unquote_val<'i>(input: &'i str) -> IResult<&'i str, &'i str> {
    preceded(
        tuple((char('\"'), space0)),
        terminated(take_until("\""), tuple((char('\"'), space0))),
    )(input)
}

fn string_val<'i>(input: &'i str) -> IResult<&'i str, Value> {
    match map_parser(take_until(","), unquote_val)(input) {
        Ok((remaining, s)) => Ok((remaining, Value::String(s.to_owned()))),
        Err(e) => Err(e),
    }
}

fn numeric_val<'i>(input: &'i str) -> IResult<&'i str, Value> {
    match tuple((space0, take_till(|x: char| !is_digit(x as u8)), space0))(dbg!(input)) {
        Ok((remaining, (d0, digits, d1))) => {
            u64::from_str_radix(dbg!(digits), 10)
                .map(|n| (remaining, Value::Numeric(n)))
                .map_err(|_e| nom::Err::Error((digits, nom::error::ErrorKind::Digit)))
        }
        Err(e) => Err(e),
    }
}

fn take_header_info_kv<'i>(input: &'i str) -> IResult<&'i str, (&'i str, Value)> {
    if input.len() == 0 {
        return Err(nom::Err::Error((input, nom::error::ErrorKind::Eof)));
    }
    match tuple((
        take_until(":"),
        char(':'),
        space0,
        alt((numeric_val, string_val)),
        space0,
        opt(tuple((char(','),space0)))
    ))(dbg!(input))
    {
        Ok((remaining_input, (k, _, _, v, _, _))) => {
            Ok((remaining_input, (k, v)))
        },
        Err(e) => Err(dbg!(e)),
    }
}

fn parse_header_info<'i>(input: &'i str) -> IResult<&'i str, HeaderInfo> {
    let (remainder, v_of_kv): (&'i str, Vec<_>) =
        all_consuming(preceded(
            tuple((char('\"'), space0)),
            terminated(
                many1(take_header_info_kv),
                tuple((space0, char('\"'),space0)))
        ))(input)?;

    let h = v_of_kv
        .into_iter()
        .try_fold(HeaderInfo::default(), |mut h, (k, v)| {
            let k = k.to_lowercase();
            match SUPPORTED.get(k.as_str()) {
                Some(ValueType::Numeric) => match k.as_str() {
                    "repeat" => h.repeat = v.as_usize(),
                    "steps" => h.steps = v.as_usize(),
                    _ => unreachable!("Must be contained in the SUPPORTED index"),
                },
                Some(ValueType::String) => {
                    let v = v.to_owned();
                    match k.as_str() {
                        "pallet" => h.pallet_name = v.as_str().to_owned(),
                        "extrinsic" => h.extrinsic = v.as_str().to_owned(),
                        _ => unreachable!("Must be contained in the SUPPORTED index"),
                    }
                }
                _ => {
                    warn!("Unknown header element type");
                }
            }
            Ok(h)
        })?;
    Ok((remainder, h))
}

fn parser_header_column(x: &str) -> IResult<&str, HeaderColumns> {
    unimplemented!("nope, not yet")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unquote<'i, 'j>()
    where
        'i: 'static,
        'i: 'j,
    {
        let res = unquote_val(r#""red""#).unwrap();
        assert_eq!(res.1, r#"red"#);
    }

    #[test]
    fn simple<'i, 'j>()
    where
        'i: 'static,
        'i: 'j,
    {
        let raw_header = vec![
            r#""Pallet: "pallet-utility", Extrinsic: "as_sub", Steps: 30, Repeat: 11""#,
            r#""A,I,time""#,
        ];

        let res = parse_header_info(raw_header[0]).unwrap();
        assert_eq!(
            res.1,
            HeaderInfo {
                pallet_name: "pallet-utility".to_owned(),
                extrinsic: "as_sub".to_owned(),
                steps: 30,
                repeat: 11,
            }
        );
    }
}
