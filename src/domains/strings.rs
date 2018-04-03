//! The string editing domain, for tackling Flashfill-style problems.
//!
//! # Examples
//!
//! ```
//! #[macro_use]
//! extern crate polytype;
//! extern crate programinduction;
//! use programinduction::{lambda, ECParams, EC};
//! use programinduction::domains::strings;
//!
//! # fn main() {
//! let dsl = strings::dsl();
//! let examples = vec![
//!     // Replace delimiter '>' with '/'
//!     (
//!         vec![strings::Space::Str("OFJQc>BLVP>eMS".to_string())],
//!         strings::Space::Str("OFJQc/BLVP/eMS".to_string()),
//!     ),
//! ];
//! let task = lambda::task_by_evaluation(
//!     strings::Evaluator,
//!     ptp!(@arrow[tp!(str), tp!(str)]),
//!     &examples,
//! );
//!
//! let ec_params = ECParams {
//!     frontier_limit: 10,
//!     search_limit_timeout: None,
//!     search_limit_description_length: Some(12.0),
//! };
//! let frontiers = dsl.explore(&ec_params, &[task]);
//! let solution = &frontiers[0].best_solution().unwrap().0;
//! assert_eq!(
//!     "(λ (join (char->str /) (split > $0)))",
//!     dsl.display(solution)
//! );
//! # }
//! ```

use itertools::Itertools;

use lambda::{Evaluator as EvaluatorT, Language, LiftedFunction};

/// The string editing [`lambda::Language`] defines the following operations:
///
/// ```ignore
/// "0":         ptp!(int)
/// "+1":        ptp!(@arrow[tp!(int), tp!(int)])
/// "-1":        ptp!(@arrow[tp!(int), tp!(int)])
/// "len":       ptp!(@arrow[tp!(str), tp!(int)])
/// "empty_str": ptp!(str)
/// "lower":     ptp!(@arrow[tp!(str), tp!(str)])
/// "upper":     ptp!(@arrow[tp!(str), tp!(str)])
/// "concat":    ptp!(@arrow[tp!(str), tp!(str), tp!(str)])
/// "slice":     ptp!(@arrow[tp!(int), tp!(int), tp!(str), tp!(str)])
/// "nth":       ptp!(@arrow[tp!(int), tp!(list(tp!(str))), tp!(str)])
/// "map":       ptp!(0, 1; @arrow[
///                  tp!(@arrow[tp!(0), tp!(1)]),
///                  tp!(list(tp!(0))),
///                  tp!(list(tp!(1))),
///              ])
/// "strip":     ptp!(@arrow[tp!(str), tp!(str)])
/// "split":     ptp!(@arrow[tp!(char), tp!(str), tp!(list(tp!(str)))])
/// "join":      ptp!(@arrow[tp!(str), tp!(list(tp!(str))), tp!(str)])
/// "char->str": ptp!(@arrow[tp!(char), tp!(str)])
/// "space":     ptp!(char)
/// ".":         ptp!(char)
/// ",":         ptp!(char)
/// "<":         ptp!(char)
/// ">":         ptp!(char)
/// "/":         ptp!(char)
/// "@":         ptp!(char)
/// "-":         ptp!(char)
/// "|":         ptp!(char)
/// ```
///
/// [`lambda::Language`]: ../../lambda/struct.Language.html
pub fn dsl() -> Language {
    Language::uniform(vec![
        ("0", ptp!(int)),
        ("+1", ptp!(@arrow[tp!(int), tp!(int)])),
        ("-1", ptp!(@arrow[tp!(int), tp!(int)])),
        ("len", ptp!(@arrow[tp!(str), tp!(int)])),
        ("empty_str", ptp!(str)),
        ("lower", ptp!(@arrow[tp!(str), tp!(str)])),
        ("upper", ptp!(@arrow[tp!(str), tp!(str)])),
        ("concat", ptp!(@arrow[tp!(str), tp!(str), tp!(str)])),
        (
            "slice",
            ptp!(@arrow[tp!(int), tp!(int), tp!(str), tp!(str)]),
        ),
        ("nth", ptp!(@arrow[tp!(int), tp!(list(tp!(str))), tp!(str)])),
        (
            "map-to-nums",
            ptp!(0; @arrow[tp!(@arrow[tp!(0), tp!(int)]), tp!(list(tp!(0))), tp!(list(tp!(int)))]),
        ),
        (
            "map-to-strs",
            ptp!(0; @arrow[tp!(@arrow[tp!(0), tp!(str)]), tp!(list(tp!(0))), tp!(list(tp!(str)))]),
        ),
        ("strip", ptp!(@arrow[tp!(str), tp!(str)])),
        (
            "split",
            ptp!(@arrow[tp!(char), tp!(str), tp!(list(tp!(str)))]),
        ),
        (
            "join",
            ptp!(@arrow[tp!(str), tp!(list(tp!(str))), tp!(str)]),
        ),
        ("char->str", ptp!(@arrow[tp!(char), tp!(str)])),
        ("space", ptp!(char)),
        (".", ptp!(char)),
        (",", ptp!(char)),
        ("<", ptp!(char)),
        (">", ptp!(char)),
        ("/", ptp!(char)),
        ("@", ptp!(char)),
        ("-", ptp!(char)),
        ("|", ptp!(char)),
    ])
}

use self::Space::*;
/// All values in the strings domain can be represented in this `Space`.
#[derive(Clone)]
pub enum Space {
    Num(i32),
    Char(char),
    Str(String),
    StrList(Vec<String>),
    NumList(Vec<i32>),
    Func(LiftedFunction<Space, Evaluator>),
}
impl PartialEq for Space {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&Num(x), &Num(y)) => x == y,
            (&Char(x), &Char(y)) => x == y,
            (&Str(ref x), &Str(ref y)) => x == y,
            (&StrList(ref xs), &StrList(ref ys)) => xs == ys,
            (&NumList(ref xs), &NumList(ref ys)) => xs == ys,
            _ => false,
        }
    }
}
/// An [`Evaluator`] for the strings domain.
///
/// [`Evaluator`]: ../../lambda/trait.Evaluator.html
#[derive(Copy, Clone)]
pub struct Evaluator;
impl EvaluatorT for Evaluator {
    type Space = Space;
    fn evaluate(&self, name: &str, inps: &[Self::Space]) -> Self::Space {
        match OPERATIONS[name] {
            Op::Zero => Num(0),
            Op::Incr => match inps[0] {
                Num(x) => Num(x + 1),
                _ => unreachable!(),
            },
            Op::Decr => match inps[0] {
                Num(x) => Num(x - 1),
                _ => unreachable!(),
            },
            Op::Len => match inps[0] {
                Str(ref s) => Num(s.len() as i32),
                _ => unreachable!(),
            },
            Op::Empty => Str(String::new()),
            Op::Lower => match inps[0] {
                Str(ref s) => Str(s.to_lowercase()),
                _ => unreachable!(),
            },
            Op::Upper => match inps[0] {
                Str(ref s) => Str(s.to_uppercase()),
                _ => unreachable!(),
            },
            Op::Concat => match (&inps[0], &inps[1]) {
                (&Str(ref x), &Str(ref y)) => {
                    let mut s = x.to_string();
                    s.push_str(y);
                    Str(s)
                }
                _ => unreachable!(),
            },
            Op::Slice => match (&inps[0], &inps[1], &inps[2]) {
                (&Num(x), &Num(y), &Str(ref s)) => {
                    Str(s.chars().skip(x as usize).take((y - x) as usize).collect())
                }
                _ => unreachable!(),
            },
            Op::Nth => match (&inps[0], &inps[1]) {
                (&Num(x), &StrList(ref ss)) => {
                    Str(ss.get(x as usize).cloned().unwrap_or_else(String::new))
                }
                _ => unreachable!(),
            },
            Op::MapToNums => match (&inps[0], &inps[1]) {
                (&Func(ref f), &NumList(ref xs)) => NumList(
                    xs.into_iter()
                        .cloned()
                        .map(|x| match f.eval(&[Num(x)]) {
                            Num(y) => y,
                            _ => panic!("map given invalid function"),
                        })
                        .collect(),
                ),
                (&Func(ref f), &StrList(ref xs)) => NumList(
                    xs.into_iter()
                        .cloned()
                        .map(|x| match f.eval(&[Str(x)]) {
                            Num(y) => y,
                            _ => panic!("map given invalid function"),
                        })
                        .collect(),
                ),
                _ => unreachable!(),
            },
            Op::MapToStrs => match (&inps[0], &inps[1]) {
                (&Func(ref f), &NumList(ref xs)) => StrList(
                    xs.into_iter()
                        .cloned()
                        .map(|x| match f.eval(&[Num(x)]) {
                            Str(y) => y,
                            _ => panic!("map given invalid function"),
                        })
                        .collect(),
                ),
                (&Func(ref f), &StrList(ref xs)) => StrList(
                    xs.into_iter()
                        .cloned()
                        .map(|x| match f.eval(&[Str(x)]) {
                            Str(y) => y,
                            _ => panic!("map given invalid function"),
                        })
                        .collect(),
                ),
                _ => unreachable!(),
            },
            Op::Strip => match inps[0] {
                Str(ref s) => Str(s.trim().to_string()),
                _ => unreachable!(),
            },
            Op::Split => match (&inps[0], &inps[1]) {
                (&Char(c), &Str(ref s)) => StrList(s.split(c).map(str::to_string).collect()),
                _ => unreachable!(),
            },
            Op::Join => match (&inps[0], &inps[1]) {
                (&Str(ref delim), &StrList(ref ss)) => Str(ss.iter().join(delim)),
                _ => unreachable!(),
            },
            Op::CharToStr => match inps[0] {
                Char(c) => Str(c.to_string()),
                _ => unreachable!(),
            },
            Op::CharSpace => Char(' '),
            Op::CharDot => Char('.'),
            Op::CharComma => Char(','),
            Op::CharLess => Char('<'),
            Op::CharGreater => Char('>'),
            Op::CharSlash => Char('/'),
            Op::CharAt => Char('@'),
            Op::CharDash => Char('-'),
            Op::CharPipe => Char('|'),
        }
    }
    fn lift(&self, f: LiftedFunction<Self::Space, Self>) -> Result<Self::Space, ()> {
        Ok(Func(f))
    }
}

/// Using an enum with a hashmap will be much faster than string comparisons.
enum Op {
    Zero,
    Incr,
    Decr,
    Len,
    Empty,
    Lower,
    Upper,
    Concat,
    Slice,
    Nth,
    MapToNums,
    MapToStrs,
    Strip,
    Split,
    Join,
    CharToStr,
    CharSpace,
    CharDot,
    CharComma,
    CharLess,
    CharGreater,
    CharSlash,
    CharAt,
    CharDash,
    CharPipe,
}

lazy_static! {
    static ref OPERATIONS: ::std::collections::HashMap<&'static str, Op> = hashmap!{
        "0" => Op::Zero,
        "+1" => Op::Incr,
        "-1" => Op::Decr,
        "len" => Op::Len,
        "empty_str" => Op::Empty,
        "lower" => Op::Lower,
        "upper" => Op::Upper,
        "concat" => Op::Concat,
        "slice" => Op::Slice,
        "nth" => Op::Nth,
        "map-to-nums" => Op::MapToNums,
        "map-to-strs" => Op::MapToStrs,
        "strip" => Op::Strip,
        "split" => Op::Split,
        "join" => Op::Join,
        "char->str" => Op::CharToStr,
        "space" => Op::CharSpace,
        "." => Op::CharDot,
        "," => Op::CharComma,
        "<" => Op::CharLess,
        ">" => Op::CharGreater,
        "/" => Op::CharSlash,
        "@" => Op::CharAt,
        "-" => Op::CharDash,
        "|" => Op::CharPipe,
    };
}
