#![allow(unused_imports)]

use combine::error::{ConsumedResult, FastResult};
use combine::stream::{FullRangeStream, Resetable, Stream, StreamOnce};
use combine::{
    attempt, choice,
    combinator::{any_send_partial_state, AnySendPartialState},
    error::{ParseError, StreamError},
    look_ahead, many, optional, position,
    parser::{
        char::{char, digit, space},
        range::{range, recognize, take, take_while},
    },
    satisfy, skip_count_min_max, skip_many, skip_many1,
    stream::{easy, PartialStream, RangeStream, StreamErrorFor},
    value, Parser,
};

fn myparser<'a, I>(
) -> impl Parser<Input = I, Output = (), PartialState = AnySendPartialState> + 'a
where
    I: RangeStream<Item = char, Range = &'a str> + 'a,
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{

    any_send_partial_state(
            skip_count_min_max(1, 2, ( char('_'), char('1') )).skip(char('.')) // A
            //skip_many1(              ( char('_'), char('1') )).skip(char('.')) // B
    )
}

/// Just a convenience function to format an easy::Error better
fn make_err_readable<'a>(
    e: easy::Errors<char, &'a str, combine::stream::PointerOffset>,
    src: &str,
) -> String {
    let e = e.map_position(|p| p.translate_position(&src[..]));
    format!("  {}\nIn input: `{}`", e, src)
}


/// Calls the parser twice, first with the string from `step1`, then with the remaining
/// string from `step1` plus `step2`.
///
/// On Success it returns Ok(Some(())).
/// If the parsing could not complete, it returns Ok(None).
fn decode2(step1: &str, step2 : &str) -> Result<Option<()>, String> {
    let mut partial_state: AnySendPartialState = Default::default();

    let stream1 = easy::Stream(PartialStream(&step1[..]));
    let (opt, removed_len) = combine::stream::decode(myparser(), stream1, &mut partial_state)
        .map_err(|e| make_err_readable(e, &step1))?;

    if let Some(output) = opt { return Ok(Some(output)) }

    let mut step1step2 = String::from(&step1[removed_len..]);
    step1step2.push_str(step2);
    let stream2 = easy::Stream(PartialStream(&step1step2[..]));
    let (opt, _removed_len) = combine::stream::decode(myparser(), stream2, &mut partial_state)
        .map_err(|e| make_err_readable(e, &step1step2))?;

    if let Some(output) = opt { return Ok(Some(output)) }

    return Ok(None);
}

fn main() {}



#[test]
fn test_invalid() {
    assert!(decode2("_.", "")
        .unwrap_err()
        .contains("Unexpected"));
}

#[test]
fn test_no_split() {
    assert_eq!(
        Ok(Some(())),
        decode2("_1.", "")
    );
}

#[test]
fn test_no_split_2() {
    assert_eq!(
        Ok(Some(())),
        decode2("", "_1.")
    );
}

#[test]
fn test_split_after_1() {
    assert_eq!(
        Ok(Some(())),
        decode2("_1", ".")
    );
}

#[test]
fn test_split_before_1() {
    assert_eq!(
        Ok(Some(())),
        decode2("_", "1.")
    );
}

