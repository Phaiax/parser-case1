#![allow(unused_imports)]

use combine::error::{ConsumedResult, FastResult};
use combine::stream::{FullRangeStream, Resetable, Stream, StreamOnce};
use combine::{
    attempt, choice,
    combinator::{any_send_partial_state, AnySendPartialState},
    error::{ParseError, StreamError},
    look_ahead, many, optional,
    parser::{
        byte::{byte, digit, space},
        range::{range, recognize, take, take_while},
    },
    satisfy, skip_count_min_max, skip_many, skip_many1,
    stream::{easy, PartialStream, RangeStream, StreamErrorFor},
    value, Parser,
};

fn base_protocol<'a, I>(
) -> impl Parser<Input = I, Output = Vec<u8>, PartialState = AnySendPartialState> + 'a
where
    I: RangeStream<Item = u8, Range = &'a [u8]> + FullRangeStream + 'a,
    // Necessary due to rust-lang/rust#24159
    I::Error: ParseError<I::Item, I::Range, I::Position>,
{
    let foobar = range(&b"foobar"[..])
        .with(recognize(skip_many1(digit())).skip(range(&b"\r\n"[..])))
        .map(|_| ());
    let foobaz = range(&b"foobaz"[..]).skip(range(&b"\r\n"[..])).map(|_| ());

    any_send_partial_state(
        (
            //content_length,
            //optional(content_type),
            //skip_many(range(&b"\r\n"[..])),
            //skip_many1(recognize(look_ahead(satisfy(|t| t != b'\r'))).with(choice((content_length, content_type)))),
            //skip_count_min_max(1, 2, (optional(foobar), optional(foobaz))),
            skip_count_min_max(1, 2, (attempt(foobar), attempt(foobaz))),
            //skip_many((optional(foobar), optional(foobaz))),
            range(&b"\r\n"[..]).map(|_| {
                println!("got \\r\\n");
                ()
            }),
        )
            .then_partial(move |&mut _| {
                take_while(|t| t != b'\r')
                    .map(|bytes: &[u8]| bytes.to_owned())
                    .skip(range(&b"\r\n"[..]))
            }),
    )
}


fn make_err_readable<'a>(e:easy::Errors<u8, &'a[u8], combine::stream::PointerOffset>, src:&[u8]) -> String  {

    // Make any byte slice reference in the error displayable
    let e = e.map_range(|r| {
                std::str::from_utf8(r).ok().map_or_else(
                    || format!("{:?}", r), // default
                    |s| s.to_string(),
                )
            })
            .map_position(|p| p.translate_position(&src[..]))
            .map_token(|t| std::char::from_u32(t as u32).unwrap_or('?'));
    format!("{}\nIn input: `{}`", e, std::str::from_utf8(src).unwrap())
}

fn decode(src: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let mut partial_state: AnySendPartialState = Default::default();
    let stream = easy::Stream(PartialStream(&src[..]));

    let (opt, removed_len) = combine::stream::decode(base_protocol(), stream, &mut partial_state)
        .map_err(|e| make_err_readable(e, src))?;
    println!("removed: {} bytes", removed_len);
    //src.split_to(removed_len);

    match opt {
        None => Ok(None),
        Some(output) => Ok(Some(output)),
    }
}

fn decode_partial(src: &[(&[u8])]) -> Result<Option<Vec<u8>>, String> {
    let mut partial_state: AnySendPartialState = Default::default();
    //let src = src.into()

    let mut current_src = vec![];
    for srcp in src {
        current_src.extend(srcp.iter());
        println!("{:?}", std::str::from_utf8(&current_src));

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
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode(b"foobar1\r\nfoobaz\r\n\r\nabcdefg\r\n")
    );
    assert_eq!(
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode(b"foobaz\r\nfoobar1\r\n\r\nabcdefg\r\n")
    );
    assert_eq!(
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode(b"foobar1\r\n\r\nabcdefg\r\n")
    );
    assert_eq!(
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode(b"foobaz\r\n\r\nabcdefg\r\n")
    );
    assert!(decode(b"foobac\r\n\r\nabcdefg\r\n")
        .unwrap_err()
        .contains("Unexpected"));
    assert!(decode(b"foobar1\r\nj\r\nabcdefg\r\n")
        .unwrap_err()
        .contains("Unexpected `j`"));

    assert_eq!(
        Ok(None),
        decode(b"foobar1\r\nfoobaz\r\n\r\nabcdefg")
    );

    assert_eq!(
        Ok(None),
        decode(b"foobar1\r\nfoobaz\r\n\r\n")
    );

    assert_eq!(
        Ok(None),
        decode(b"foobar1\r\nfoobaz\r\n")
    );

    assert_eq!(
        Ok(None),
        decode(b"foobar1\r\nfoobaz")
    );

    assert_eq!(
        Ok(None),
        decode(b"foobaz\r\nfoob")
    );

    assert!(
        decode(b"foobaz\r\nfoobcc").is_err()
    );

    assert_eq!(
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode_partial(&[&b"foobar1\r\nfoobaz\r\n\r\nabcdefg\r\n"[..]][..])
    );

    assert_eq!(
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode_partial(&[&b"foobar12\r\n"[..], &b"foobaz\r\n\r\nabcdefg\r\n"[..]][..])
    );

    assert_eq!(
        Ok(Some(b"abcdefg".as_ref().to_owned())),
        decode_partial(&[&b"foobar1"[..], &b"2\r\nfoobaz\r\n\r\nabcdefg\r\n"[..]][..])
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
