#![allow(unused_imports)]

use combine::error::{ConsumedResult, FastResult};
use combine::stream::{FullRangeStream, Resetable, Stream, StreamOnce};
use combine::{
    attempt, choice,
    combinator::{any_send_partial_state, AnySendPartialState},
    error::{ParseError, StreamError},
    look_ahead, many, optional, position,
    parser::{
        char::{digit, space},
        range::{range, recognize, take, take_while},
    },
    satisfy, skip_count_min_max, skip_many, skip_many1,
    stream::{easy, PartialStream, RangeStream, StreamErrorFor},
    value, Parser,
};

/// This function is the parser. It should parse things like
///
/// foobar<number> and foobaz are headers. Each header is followed by `\r\n` and
/// the whole headerblock is followed by another `\r\n`. Then starts the data
/// block which is followed by another `\r\n`.
///
/// Examples:
///
///  - `foobar1\r\nfoobaz\r\n\r\n some arbitrary text`
///  - `foobaz\r\nfoobar1234\r\n\r\n some arbitrary text`
///
/// The parser is usable with a  partial stream aka can resume.
fn myparser<'a, I>(
) -> impl Parser<Input = I, Output = String, PartialState = AnySendPartialState> + 'a
where
    I: RangeStream<Item = char, Range = &'a str> + 'a,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    // P1.with(P2) Discards the output of P1 and returns the output of P2
    // recognize(P) Returns the data that P parsed, but unparsed (if P is skipping)
    // P1.and_then(F:FnMut) Processes the output of P1 and may (in contrast to .map()) fail
    // P.then_partial(F:FnMut) Abhängig vom outpt von P einen neuen Parser generieren, der weitermacht

    let foobar  =
       recognize(range(&"f"[..]))
       .with(recognize(skip_many1(digit())).map(|_| ()).skip(range(&"\r\n"[..])))
       .map(|_| ());
    //let foobaz = range(&"foobaz"[..]).map(|_| ()).skip(range(&"\r\n"[..]));

    any_send_partial_state(
        (
            //skip_many1(choice(( optional(foobar), optional(foobaz)))), // takes forever, that's understandable
            //skip_many1(( optional(foobar), optional(foobaz))), // takes forever, that's understandable
            //skip_count_min_max(1, 2, (optional(foobar), optional(foobaz))), // works almost


            //skip_count_min_max(1, 2, (attempt(foobar), attempt(foobaz))), // does not work good
            //skip_many1( ( attempt(foobar), attempt(foobaz))), // nooope

//  <<<<<<<<<<
// This is an interesting pair: Why does the first fail with partial parsing
// but the second one succeeds?
            skip_count_min_max(0, 3, foobar), // works almost, execept test_partial_split_inbetween_number_of_foobar
            //skip_many1(choice((foobar, foobaz))), // perfect
// >>>>>>>>>>>

            //skip_many1(choice(( attempt(foobar), attempt(foobaz)))), // perfect
            range(&"\r\n"[..]).map(|_| { // seems to be neccessary
                //println!("got \\r\\n");
                ()
            }),
        )
            .map(|_| "".to_string()),
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
fn decode(src: &str) -> Result<Option<String>, String> {

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
fn decode_partial(src: &[&str]) -> Result<Option<String>, String> {
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

// #[test]
// fn test_foobar_first() {
//     assert_eq!(
//         Ok(Some("abcdefg".to_string())),
//         decode("foobar1\r\nfoobaz\r\n\r\nabcdefg\r\n")
//     );
// }

// #[test]
// fn test1_foobaz_first() {
//     assert_eq!(
//         Ok(Some("abcdefg".to_string())),
//         decode("foobaz\r\nfoobar1\r\n\r\nabcdefg\r\n")
//     );
// }

#[test]
fn test_no_foobaz() {
    assert_eq!(
        Ok(Some("".to_string())),
        decode("f1\r\n\r\nabcd")
    );
}


#[test]
fn test_invalid_header() {
    assert!(decode("foobac\r\n\r\nabcdefg\r\n")
        .unwrap_err()
        .contains("Unexpected"));
}

#[test]
fn test_invalid_header_after_valid_header() {
    assert!(decode("f1\r\nj\r\nabcdefg\r\n")
        .unwrap_err()
        .contains("Unexpected `j`"));
}


#[test]
fn test_decode_partial_does_same_as_decode() {
    assert_eq!(
        Ok(Some("".to_string())),
        decode_partial(&["f1\r\n\r\nabcdefg\r\n"][..])
    );
}

#[test]
fn test_partial_split_after_number_of_foobar() {
    assert_eq!(
        Ok(Some("".to_string())),
        decode_partial(&["f12\r\n", "\r\nabcdefg\r\n"][..])
    );
}

#[test]
fn test_partial_split_inbetween_number_of_foobar() {
    assert_eq!(
        Ok(Some("".to_string())),
        decode_partial(&["f1", "2\r\n\r\nabcdefg\r\n"][..])
    );
}
