use std::num::ParseFloatError;

use num_bigint::{BigInt, ParseBigIntError};
use num_traits::Num;

use nom::{
    IResult, Parser,
    Err::{Incomplete, Error, Failure},
    branch::alt,
    bytes::complete::{escaped_transform, is_not, tag},
    character::complete::{char, none_of, one_of, multispace0},
    combinator::{map, map_res, opt, recognize, value, verify},
    error::{ParseError, FromExternalError, ContextError, VerboseError},
    multi::{many0, many1, separated_list0},
    sequence::{preceded, terminated, tuple},
};

trait CompleteError<'a>:
    ParseError<&'a str> +
    ContextError<&'a str> +
    FromExternalError<&'a str, ParseBigIntError> +
    FromExternalError<&'a str, ParseFloatError> {}

impl<'a, T> CompleteError<'a> for T
    where T:
    ParseError<&'a str> +
    ContextError<&'a str> +
    FromExternalError<&'a str, ParseBigIntError> +
    FromExternalError<&'a str, ParseFloatError> {}

#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Integer(i64),
    BigInteger(BigInt),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Identifier(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringElement {
    Raw(String),
    Interpolate(AstNode),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListElement {
    Singleton(AstNode),
}

#[derive(Debug, Clone, PartialEq)]
pub enum MapElement {
    Singleton(AstNode, AstNode),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Literal(Object),
    String(Vec<StringElement>),
    Identifier(String),
    List(Vec<ListElement>),
    Map(Vec<MapElement>),
    Let(Vec<(Binding, AstNode)>, Box<AstNode>),
}

impl AstNode {
    fn integer(value: i64) -> AstNode { AstNode::Literal(Object::Integer(value)) }
    fn big_integer(value: BigInt) -> AstNode { AstNode::Literal(Object::BigInteger(value)) }
    fn float(value: f64) -> AstNode { AstNode::Literal(Object::Float(value)) }
    fn boolean(value: bool) -> AstNode { AstNode::Literal(Object::Boolean(value)) }
    fn null() -> AstNode { AstNode::Literal(Object::Null) }

    fn string(value: Vec<StringElement>) -> AstNode {
        if value.len() == 0 {
            AstNode::Literal(Object::String("".to_string()))
        } else if value.len() == 1 {
            match &value[0] {
                StringElement::Raw(val) => AstNode::Literal(Object::String(val.clone())),
                _ => AstNode::String(value)
            }
        } else {
            AstNode::String(value)
        }
    }
}

fn postpad<I, O, E: ParseError<I>, F>(
    parser: F,
) -> impl FnMut(I) -> IResult<I, O, E>
where
    F: Parser<I, O, E>,
    I: Clone + nom::InputTakeAtPosition,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone
{
    terminated(parser, multispace0)
}

static KEYWORDS: [&'static str; 12] = [
    "for",
    "if",
    "then",
    "else",
    "let",
    "in",
    "true",
    "false",
    "null",
    "and",
    "or",
    "not",
];

fn identifier<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    // map(
        verify(
            is_not("-+/*[](){}\"\' \t\n\r"),
            |out: &str| !KEYWORDS.contains(&out),
        )(input)
        // out.to_string()
        // |out: &str| AstNode::Identifier(out.to_string())
    // )(input)
}

fn map_identifier<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    is_not("=$\"\' \t\n\r")(input)
}

fn decimal<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, &'a str, E> {
    recognize(tuple((
        one_of("0123456789"),
        many0(one_of("0123456789_")),
    )))(input)
}

fn exponent<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&str, &str, E> {
    recognize(tuple((
        one_of("eE"),
        opt(one_of("+-")),
        decimal,
    )))(input)
}

fn integer<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    map_res(
        decimal,
        |out: &'a str| {
            let s = out.replace("_", "");
            i64::from_str_radix(s.as_str(), 10).map_or_else(
                |_| { BigInt::from_str_radix(s.as_str(), 10).map(AstNode::big_integer) },
                |val| Ok(AstNode::integer(val)),
            )
        }
    )(input)
}

fn float<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    map_res(
        alt((
            recognize(tuple((
                decimal,
                char('.'),
                opt(decimal),
                opt(exponent),
            ))),
            recognize(tuple((
                char('.'),
                decimal,
                opt(exponent),
            ))),
            recognize(tuple((
                decimal,
                exponent,
            ))),
        )),
        |out: &str| { out.replace("_", "").parse::<f64>().map(AstNode::float) }
    )(input)
}

fn string_data<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, StringElement, E> {
    map(
        escaped_transform(
            recognize(many1(none_of("\"\\$"))),
            '\\',
            alt((
                value("\"", tag("\"")),
                value("\\", tag("\\")),
            )),
        ),
        StringElement::Raw,
    )(input)
}

fn string_interp<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, StringElement, E> {
    map(
        preceded(
            postpad(tag("${")),
            terminated(
                expression,
                char('}'),
            ),
        ),
        StringElement::Interpolate,
    )(input)
}

fn string<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    map(
        preceded(
            char('\"'),
            terminated(
                many0(alt((string_interp, string_data))),
                char('\"'),
            ),
        ),
        AstNode::string
    )(input)
}

fn boolean<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    alt((
        value(AstNode::boolean(true), tag("true")),
        value(AstNode::boolean(false), tag("false")),
    ))(input)
}

fn null<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    value(AstNode::null(), tag("null"))(input)
}

fn atomic<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    alt((
        null,
        boolean,
        float,
        integer,
        string,
    ))(input)
}

fn list_element<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, ListElement, E> {
    alt((
        map(expression, ListElement::Singleton),
    ))(input)
}

fn list<'a, E: CompleteError<'a>>(
    input: &'a str
) -> IResult<&'a str, AstNode, E> {
    map(
        preceded(
            postpad(char('[')),
            terminated(
                separated_list0(
                    postpad(char(',')),
                    list_element
                ),
                tuple((
                    opt(postpad(char(','))),
                    char(']')
                )),
            ),
        ),
        AstNode::List,
    )(input)
}

fn map_element<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, MapElement, E> {
    alt((
        map(
            tuple((
                terminated(
                    postpad(map_identifier),
                    postpad(char('=')),
                ),
                expression,
            )),
            |(key, value)| MapElement::Singleton({
                let value = key.to_string(); AstNode::Literal(Object::String(value.to_string())) }, value),
        ),
    ))(input)
}

fn mapping<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    map(
        preceded(
            postpad(char('{')),
            terminated(
                separated_list0(
                    postpad(char(',')),
                    map_element,
                ),
                tuple((
                    opt(postpad(char(','))),
                    char('}'),
                )),
            ),
        ),
        AstNode::Map,
    )(input)
}

fn postfixable<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    postpad(alt((
        atomic,
        map(identifier, |out: &str| AstNode::Identifier(out.to_string())),
        list,
        mapping,
    )))(input)
}

fn binding<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, Binding, E> {
    postpad(alt((
        map(identifier, |out: &str| Binding::Identifier(out.to_string())),
    )))(input)
}

fn let_block<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    map(
        tuple((
            many1(
                tuple((
                    preceded(
                        postpad(tag("let")),
                        binding,
                    ),
                    preceded(
                        postpad(tag("=")),
                        expression,
                    ),
                )),
            ),
            preceded(
                postpad(tag("in")),
                expression,
            ),
        )),
        |(bindings, expr)| AstNode::Let(bindings, Box::new(expr)),
    )(input)
}

fn composite<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    alt((
        let_block,
    ))(input)
}

fn expression<'a, E: CompleteError<'a>>(
    input: &'a str,
) -> IResult<&'a str, AstNode, E> {
    alt((
        composite,
        postfixable,
    ))(input)
}

pub fn parse(input: &str) -> Result<AstNode, String> {
    expression::<VerboseError<&str>>(input).map_or_else(
        |err| match err {
            Incomplete(_) => Err("incomplete input".to_string()),
            Error(e) | Failure(e) => Err(format!("{:#?}", e)),
        },
        |(remaining, node)| if remaining.len() > 0 {
            Err(format!("unconsumed input: {}", remaining))
        } else {
            Ok(node)
        }
    )
}

#[cfg(test)]
mod tests;
