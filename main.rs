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

    let foobar  = ( char('_'), char('1') );

    any_send_partial_state(
            skip_count_min_max(1, 2, foobar) // A
            //skip_many1(foobar)             // B
            .skip(char('.')) // seems to be neccessary
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

/// A Decode function which tries to parse the given data once.
/// If the parsing could not complete, it returns Ok(None).
/// On Success it returns Ok(Some(data part)).
fn decode(src: &str) -> Result<Option<()>, String> {

    let mut partial_state: AnySendPartialState = Default::default();
    let stream = easy::Stream(PartialStream(&src[..]));

    let (opt, removed_len) = combine::stream::decode(myparser(), stream, &mut partial_state)
        .map_err(|e| make_err_readable(e, src))?;

    if removed_len != src.len() {
        println!(
            "  Parser left {} bytes unparsed: {:?}",
            src.len() - removed_len,
            &src[removed_len..]
        );
    }

    match opt {
        None => Ok(None),
        Some(output) => Ok(Some(output)),
    }
}

/// Same as decode, but it calls the parser several times with the same
/// partial state.
/// On each call, the input string is extended with the next element from `src`.
///
/// It returns Ok(None), if after the last parsing round, there still was neither
/// an error nor a successful parsing.
fn decode_partial(src: &[&str]) -> Result<Option<()>, String> {
    let mut partial_state: AnySendPartialState = Default::default();

    let mut current_src = String::new();
    for srcp in src.iter() {
        let _s: &str = srcp;
        current_src.push_str(srcp);
        println!("  Input for current round: {:?}", current_src);

        let stream = easy::Stream(PartialStream(&current_src[..]));
        let (opt, removed_len) = combine::stream::decode(myparser(), stream, &mut partial_state)
            .map_err(|e| make_err_readable(e, &current_src))?;
        println!("  removed: {} bytes", removed_len);

        current_src = current_src.split_off(removed_len);

        match opt {
            None => continue, //Ok(None),
            Some(output) => return Ok(Some(output)),
        }
    }
    return Ok(None);
}

fn main() {}


#[test]
fn test_no_foobaz() {
    assert_eq!(
        Ok(Some(())),
        decode("_1.")
    );
}


#[test]
fn test_invalid_header() {
    assert!(decode("_.")
        .unwrap_err()
        .contains("Unexpected"));
}



#[test]
fn test_decode_partial_does_same_as_decode() {
    assert_eq!(
        Ok(Some(())),
        decode_partial(&["_1."][..])
    );
}

#[test]
fn test_partial_split_after_header() {
    assert_eq!(
        Ok(Some(())),
        decode_partial(&["_1.", ""][..])
    );
}

#[test]
fn test_partial_split_after_number_of_foobar() {
    assert_eq!(
        Ok(Some(())),
        decode_partial(&["_1", "."][..])
    );
}

// #[test]
// fn test_partial_split_inbetween_number_of_foobar() {
//     assert_eq!(
//         Ok(Some(())),
//         decode_partial(&["_1", "2."][..])
//     );
// }

#[test]
fn test_partial_split_before_number_of_foobar() {
    assert_eq!(
        Ok(Some(())),
        decode_partial(&["_", "1."][..])
    );
}
