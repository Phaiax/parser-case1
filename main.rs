#![allow(unused_imports)]

use combine::error::{ConsumedResult, FastResult};
use combine::stream::{FullRangeStream, Resetable, Stream, StreamOnce};
use combine::{
    attempt, choice,
    combinator::{any_send_partial_state, AnySendPartialState},
    error::{ParseError, StreamError},
    look_ahead, many, optional,
    parser::{
        char::{digit, space},
        range::{range, recognize, take, take_while},
    },
    satisfy, skip_count_min_max, skip_many, skip_many1,
    stream::{easy, PartialStream, RangeStream, StreamErrorFor},
    value, Parser,
};

fn base_protocol<'a, I>(
) -> impl Parser<Input = I, Output = String, PartialState = AnySendPartialState> + 'a
where
    I: RangeStream<Item = char, Range = &'a str> + FullRangeStream + 'a,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let foobar = range(&"foobar"[..])
        .with(recognize(skip_many1(digit())).skip(range(&"\r\n"[..])))
        .map(|_| ());
    let foobaz = range(&"foobaz"[..]).skip(range(&"\r\n"[..])).map(|_| ());

    any_send_partial_state(
        (
            //content_length,
            //optional(content_type),
            //skip_many(range(&b"\r\n"[..])),
            //skip_many1(recognize(look_ahead(satisfy(|t| t != b'\r'))).with(choice((content_length, content_type)))),
            skip_count_min_max(1, 2, (optional(foobar), optional(foobaz))),
            //skip_count_min_max(1, 2, (attempt(foobar), attempt(foobaz))),
            //skip_many((optional(foobar), optional(foobaz))),
            range(&"\r\n"[..]).map(|_| {
                //println!("got \\r\\n");
                ()
            }),
        )
            .then_partial(move |&mut _| {
                take_while(|t : char| t != '\r')
                    .map(|bytes: &str| bytes.to_owned())
                    .skip(range(&"\r\n"[..]))
            }),
    )
}


fn make_err_readable<'a>(e:easy::Errors<char, &'a str, combine::stream::PointerOffset>, src:&str) -> String  {

    // Make any byte slice reference in the error displayable
    let e = e.map_position(|p| p.translate_position(&src[..]));
    format!("{}\nIn input: `{}`", e, src)
}

fn decode(src: &str) -> Result<Option<String>, String> {
    println!("--- Test start");

    let mut partial_state: AnySendPartialState = Default::default();
    let stream = easy::Stream(PartialStream(&src[..]));

    let (opt, removed_len) = combine::stream::decode(base_protocol(), stream, &mut partial_state)
        .map_err(|e| make_err_readable(e, src))?;
    //println!("removed: {} bytes", removed_len);
    //src.split_to(removed_len);

    if removed_len != src.len() {
        println!("Parser left {} bytes unparsed: {:?}", src.len() - removed_len, &src[removed_len..]);

    }

    match opt {
        None => Ok(None),
        Some(output) => Ok(Some(output)),
    }
}

fn decode_partial(src: &[&str]) -> Result<Option<String>, String> {
    let mut partial_state: AnySendPartialState = Default::default();
    //let src = src.into()

    let mut current_src = String::new();
    for srcp in src.iter() {
        let _s : &str = srcp;
        current_src.push_str(srcp);
        println!("{:?}", current_src);

        let stream = easy::Stream(PartialStream(&current_src[..]));
        let (opt, removed_len) = combine::stream::decode(base_protocol(), stream, &mut partial_state)
            .map_err(|e| make_err_readable(e, &current_src))?;
        println!("removed: {} bytes", removed_len);

        current_src = current_src.split_off(removed_len);

        match opt {
            None => continue, //Ok(None),
            Some(output) => return Ok(Some(output)),
        }
    }
    //src.split_to(removed_len);
    return Ok(None);
}


fn main() {
    // Parser for `Content-Type: application/vscode-jsonrpc; charset=utf-8`
    // that returns ("application/vscode-jsonrpc", "utf-8")
    // P1.with(P2) Discards the output of P1 and returns the output of P2
    // recognize(P) Returns the data that P parsed, but unparsed (if P is skipping)
    // P1.and_then(F:FnMut) Processes the output of P1 and may (in contrast to .map()) fail
    // P.then_partial(F:FnMut) Abhängig vom outpt von P einen neuen Parser generieren, der weitermacht

    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode("foobar1\r\nfoobaz\r\n\r\nabcdefg\r\n")
    );
    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode("foobaz\r\nfoobar1\r\n\r\nabcdefg\r\n")
    );
    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode("foobar1\r\n\r\nabcdefg\r\n")
    );
    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode("foobaz\r\n\r\nabcdefg\r\n")
    );
    assert!(decode("foobac\r\n\r\nabcdefg\r\n")
        .unwrap_err()
        .contains("Unexpected"));
    assert!(decode("foobar1\r\nj\r\nabcdefg\r\n")
        .unwrap_err()
        .contains("Unexpected `j`"));

    assert_eq!(
        Ok(None),
        decode("foobar1\r\nfoobaz\r\n\r\nabcdefg")
    );

    assert_eq!(
        Ok(None),
        decode("foobar1\r\nfoobaz\r\n\r\n")
    );

    assert_eq!(
        Ok(None),
        decode("foobar1\r\nfoobaz\r\n")
    );

    assert_eq!(
        Ok(None),
        decode("foobar1\r\nfoobaz")
    );

    assert_eq!(
        Ok(None),
        decode("foobaz\r\nfoo")
    );

    assert!(
        decode("foobaz\r\nfoobcc").is_err()
    );

    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode_partial(&[&"foobar1\r\nfoobaz\r\n\r\nabcdefg\r\n"[..]][..])
    );

    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode_partial(&[&"foobar12\r\n"[..], &"foobaz\r\n\r\nabcdefg\r\n"[..]][..])
    );

    assert_eq!(
        Ok(Some("abcdefg".to_string())),
        decode_partial(&[&"foobar1"[..], &"2\r\nfoobaz\r\n\r\nabcdefg\r\n"[..]][..])
    );

    // let mut a: BytesMut = "Content-Length: 0\r\n\r\n".into();
    // let mut decoder = LanguageServerDecoder::new();
    // let r = decoder.decode(&mut a);
    // assert_eq!(Ok(Some("".to_string())), r);

    // let mut a: BytesMut = "Content-Length: 2\r\n\r\n{}asc".into();
    // //let mut decoder = LanguageServerDecoder::new();
    // assert_eq!(Ok(Some("{}".to_string())), decoder.decode(&mut a));

    // let mut a: BytesMut = "Content-Length: 3\r\n\r\n{}".into();
    // //let mut decoder = LanguageServerDecoder::new();
    // assert_eq!(Ok(None), decoder.decode(&mut a));

    // let mut a: BytesMut = "Content-Length: 0\r\nf\r\n".into();
    // let mut decoder = LanguageServerDecoder::new();
    // assert!(decoder
    //     .decode(&mut a)
    //     .unwrap_err()
    //     .msg
    //     .contains("Unexpected `f`"));
}
