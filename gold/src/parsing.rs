use std::num::{ParseFloatError, ParseIntError};
use std::str::FromStr;

use num_bigint::{BigInt, ParseBigIntError};
use nom_locate::{LocatedSpan, position};

use nom::{
    IResult, Parser,
    Err::{Incomplete, Error, Failure},
    branch::alt,
    bytes::complete::{escaped_transform, is_not, tag},
    character::{complete::{alpha1, char, none_of, one_of, multispace0}},
    combinator::{map, map_res, opt, recognize, value, verify},
    error::{ParseError, FromExternalError, ContextError, VerboseError},
    multi::{many0, many1, separated_list0},
    sequence::{delimited, preceded, terminated, tuple, pair},
};

use crate::ast::*;
use crate::error::{Location, Tagged};
use crate::object::{Object, Key};
use crate::traits::{Boxable, Taggable, Validatable};


type Span<'a> = LocatedSpan<&'a str>;

impl<'a> From<(Span<'a>, Span<'a>)> for Location {
    fn from((l, r): (Span<'a>, Span<'a>)) -> Self {
        Self {
            offset: l.location_offset(),
            line: l.location_line(),
            length: r.location_offset() - l.location_offset(),
        }
    }
}


trait CompleteError<'a>:
    ParseError<Span<'a>> +
    ContextError<Span<'a>> +
    FromExternalError<Span<'a>, ParseIntError> +
    FromExternalError<Span<'a>, ParseBigIntError> +
    FromExternalError<Span<'a>, ParseFloatError> {}

impl<'a, T> CompleteError<'a> for T
where T:
    ParseError<Span<'a>> +
    ContextError<Span<'a>> +
    FromExternalError<Span<'a>, ParseIntError> +
    FromExternalError<Span<'a>, ParseBigIntError> +
    FromExternalError<Span<'a>, ParseFloatError> {}


type OpCons = fn(Tagged<Expr>) -> Operator;

fn postpad<I, O, E: ParseError<I>, F>(
    parser: F,
) -> impl FnMut(I) -> IResult<I, O, E>
where
    F: Parser<I, O, E>,
    I: Clone + nom::InputTakeAtPosition,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
{
    terminated(parser, multispace0)
}

fn positioned<I, O, E: ParseError<I>, F>(
    parser: F
) -> impl FnMut(I) -> IResult<I, Tagged<O>, E>
where
    F: Parser<I, O, E>,
    I: nom::InputTake + nom::InputIter + Clone,
    O: Taggable,
    Location: From<(I, I)>,
{
    map(
        tuple((position, parser, position)),
        |(l, o, r)| o.tag((l, r)),
    )
}

fn positioned_postpad<I, O, E: ParseError<I>, F>(
    parser: F,
) -> impl FnMut(I) -> IResult<I, Tagged<O>, E>
where
    F: Parser<I, O, E>,
    I: nom::InputTakeAtPosition + nom::InputTake + nom::InputIter + Clone + nom::InputLength,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    O: Taggable,
    Location: From<(I, I)>,
{
    postpad(positioned(parser))
}


static KEYWORDS: [&'static str; 14] = [
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
    "as",
    "import",
];

fn keyword<'a, E: ParseError<Span<'a>>>(
    value: &'a str,
) -> impl FnMut(Span<'a>) -> IResult<Span<'a>, Span<'a>, E> {
    verify(
        is_not("=,;.:-+/*[](){}\"\' \t\n\r"),
        move |out: &Span<'a>| { *out.fragment() == value },
    )
}

fn identifier<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Key>, E> {
    positioned(map(
        verify(
            recognize(pair(
                alt((alpha1::<Span<'a>, E>, tag("_"))),
                opt(is_not("=.,:;-+/*[](){}\"\' \t\n\r")),
            )),
            |out: &Span<'a>| !KEYWORDS.contains(out.fragment()),
        ),
        |x| Key::new(*x.fragment()),
    ))(input)
}

fn map_identifier<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Key>, E> {
    map(
        positioned(is_not(",=:$}()\"\' \t\n\r")),
        |x| x.map(|x: Span<'a>| Key::new(x.fragment()))
    )(input)
}

fn decimal<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, &'a str, E> {
    map(
        recognize(tuple((
            one_of("0123456789"),
            many0(one_of("0123456789_")),
        ))),
        |x: Span<'a>| *x.fragment(),
    )(input)
}

fn exponent<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, &str, E> {
    map(
        recognize(tuple((
            one_of("eE"),
            opt(one_of("+-")),
            decimal,
        ))),
        |x: Span<'a>| *x.fragment(),
    )(input)
}

fn integer<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map_res(
        decimal,
        |out| {
            let s = out.replace("_", "");
            i64::from_str_radix(s.as_str(), 10).map_or_else(
                |_| { BigInt::from_str(s.as_str()).map(Expr::big_integer) },
                |val| Ok(Expr::integer(val)),
            )
        }
    ))(input)
}

fn float<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map_res(
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
        |out: Span<'a>| { (*out.fragment()).replace("_", "").parse::<f64>().map(Expr::float) }
    ))(input)
}

fn raw_string<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, String, E> {
    map(
        escaped_transform(
            recognize(many1(none_of("\"\\$"))),
            '\\',
            alt((
                value("\"", tag("\"")),
                value("\\", tag("\\")),
            )),
        ),
        |x| x.to_string(),
    )(input)
}

fn string_data<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, StringElement, E> {
    map(
        raw_string,
        StringElement::raw
    )(input)
}

fn string_interp<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, StringElement, E> {
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
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
        preceded(
            char('\"'),
            terminated(
                many0(alt((string_interp, string_data))),
                char('\"'),
            ),
        ),
        Expr::string
    ))(input)
}

fn boolean<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(alt((
        value(Expr::boolean(true), keyword("true")),
        value(Expr::boolean(false), keyword("false")),
    )))(input)
}

fn null<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(value(Expr::null(), keyword("null")))(input)
}

fn atomic<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    alt((
        null,
        boolean,
        float,
        integer,
        string,
    ))(input)
}

fn list_element<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<ListElement>, E> {
    alt((
        positioned(map(
            preceded(postpad(tag("...")), expression),
            ListElement::Splat,
        )),
        positioned(map(
            tuple((
                preceded(postpad(tag("for")), binding),
                preceded(postpad(tag("in")), expression),
                preceded(postpad(char(':')), list_element),
            )),
            |(binding, iterable, expr)| ListElement::Loop {
                binding,
                iterable,
                element: Box::new(expr),
            },
        )),
        positioned(map(
            tuple((
                preceded(postpad(tag("if")), expression),
                preceded(postpad(char(':')), list_element),
            )),
            |(condition, expr)| ListElement::Cond {
                condition,
                element: Box::new(expr),
            },
        )),
        map(expression, |x| x.wraptag(ListElement::Singleton)),
    ))(input)
}

fn list<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
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
        Expr::List,
    ))(input)
}

fn map_element<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<MapElement>, E> {
    alt((
        positioned(map(
            preceded(postpad(tag("...")), expression),
            MapElement::Splat,
        )),
        positioned(map(
            tuple((
                preceded(postpad(tag("for")), binding),
                preceded(postpad(tag("in")), expression),
                preceded(postpad(char(':')), map_element),
            )),
            |(binding, iterable, expr)| MapElement::Loop {
                binding,
                iterable,
                element: Box::new(expr),
            },
        )),
        positioned(map(
            tuple((
                preceded(postpad(tag("if")), expression),
                preceded(postpad(char(':')), map_element),
            )),
            |(condition, expr)| MapElement::Cond {
                condition,
                element: Box::new(expr)
            },
        )),
        positioned(map(
            tuple((
                terminated(
                    preceded(postpad(char('$')), expression),
                    postpad(char(':')),
                ),
                expression,
            )),
            |(key, value)| MapElement::Singleton { key, value },
        )),
        map(
            tuple((
                terminated(
                    postpad(map_identifier),
                    postpad(char(':')),
                ),
                expression,
            )),
            |(key, value)| {
                let loc = Location::from((&key, &value));
                MapElement::Singleton {
                    key: key.map(Object::from).map(Expr::Literal),
                    value,
                }.tag(loc)
            },
        ),
    ))(input)
}

fn mapping<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
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
        Expr::Map,
    ))(input)
}

fn postfixable<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    postpad(alt((
        delimited(postpad(char('(')), expression, postpad(char(')'))),
        atomic,
        map(positioned(identifier), |x| x.map(Expr::Identifier)),
        list,
        mapping,
    )))(input)
}

fn object_access<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Operator>, E> {
    map(
        tuple((
            positioned_postpad(char('.')),
            identifier,
        )),
        |(dot, out)| Operator::BinOp(
            BinOp::Index,
            out.map(Object::IntString).map(Expr::Literal).to_box(),
        ).tag((&dot, &out)),
    )(input)
}

fn object_index<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Operator>, E> {
    map(
        tuple((
            positioned_postpad(char('[')),
            expression,
            positioned(char(']')),
        )),
        |(a, expr, b)| Operator::BinOp(BinOp::Index, Box::new(expr)).tag((&a, &b)),
    )(input)
}

fn function_arg<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<ArgElement>, E> {
    positioned(alt((
        map(
            preceded(postpad(tag("...")), expression),
            ArgElement::Splat,
        ),
        map(
            tuple((
                postpad(identifier),
                preceded(
                    postpad(char(':')),
                    expression,
                ),
            )),
            |(name, expr)| ArgElement::Keyword(name, expr),
        ),
        map(
            expression,
            ArgElement::Singleton,
        ),
    )))(input)
}

fn function_call<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Operator>, E> {
    map(
        tuple((
            positioned_postpad(char('(')),
            separated_list0(
                postpad(char(',')),
                function_arg,
            ),
            positioned_postpad(char(')')),
        )),
        |(a, expr, b)| Operator::FunCall(expr).tag((&a, &b)),
    )(input)
}

fn postfixed<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    map(
        tuple((
            postfixable,
            many0(postpad(alt((
                object_access,
                object_index,
                function_call,
            )))),
        )),
        |(expr, ops)| {
            ops.into_iter().fold(
                expr,
                |expr, operator| {
                    let loc = Location::from((&expr, &operator));
                    Expr::Operator { operand: Box::new(expr), operator: operator.unwrap() }.tag(loc)
                }
            )
        },
    )(input)
}

fn prefixed<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    map(
        tuple((
            many0(alt((
                map(positioned_postpad(tag("+")), |x| x.map(|_| UnOp::Passthrough)),
                map(positioned_postpad(tag("-")), |x| x.map(|_| UnOp::ArithmeticalNegate)),
                map(positioned_postpad(keyword("not")), |x| x.map(|_| UnOp::LogicalNegate)),
            ))),
            power,
        )),
        |(ops, expr)| {
            ops.into_iter().rev().fold(
                expr,
                |expr, operator| {
                    let loc = Location::from((&operator, &expr));
                    Expr::Operator { operand: Box::new(expr), operator: Operator::UnOp(operator.unwrap()) }.tag(loc)
                },
            )
        },
    )(input)
}

fn binop<I, E: ParseError<I>, G, H>(
    operators: G,
    operand: H,
) -> impl FnMut(I) -> IResult<I, Tagged<Operator>, E>
where
    I: Clone + nom::InputTakeAtPosition + nom::InputTake + nom::InputIter + nom::InputLength,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    G: Parser<I, OpCons, E>,
    H: Parser<I, Tagged<Expr>, E>,
    Location: From<(I, I)>,
{
    map(
        tuple((
            positioned_postpad(operators),
            operand,
        )),
        |(func, expr)| {
            let loc = Location::span(func.loc(), expr.loc());
            func.as_ref()(expr).direct_tag(loc)
        },
    )
}

fn binops<I, E: ParseError<I>, G, H>(
    operators: G,
    operand: H,
    right: bool,
) -> impl FnMut(I) -> IResult<I, Tagged<Expr>, E>
where
    I: Clone + nom::InputTakeAtPosition + nom::InputLength,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    G: Parser<I, Tagged<Operator>, E>,
    H: Parser<I, Tagged<Expr>, E> + Copy,
{
    map(
        tuple((
            operand,
            many0(operators),
        )),
        move |(expr, ops)| {
            let acc = |expr: Tagged<Expr>, operator: Tagged<Operator>| {
                let loc = Location::from((&expr, &operator));
                Expr::Operator {
                    operand: Box::new(expr),
                    operator: operator.unwrap(),
                }.tag(loc)
            };
            if right {
                ops.into_iter().rev().fold(expr, acc)
            } else {
                ops.into_iter().fold(expr, acc)
            }
        },
    )
}

fn power<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    binops(
        binop(
            alt((
                value(Operator::power as OpCons, tag("^")),
            )),
            prefixed,
        ),
        postfixed,
        true,
    )(input)
}

fn lbinop<I, E: ParseError<I>, G, H>(
    operators: G,
    operands: H
) -> impl FnMut(I) -> IResult<I, Tagged<Expr>, E>
where
    I: Clone + nom::InputTakeAtPosition + nom::InputLength + nom::InputTake + nom::InputIter,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
    G: Parser<I, OpCons, E>,
    H: Parser<I, Tagged<Expr>, E> + Copy,
    Location: From<(I, I)>,
{
    binops(binop(operators, operands), operands, false)
}

fn product<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    lbinop(
        alt((
            value(Operator::multiply as OpCons, tag("*")),
            value(Operator::integer_divide as OpCons, tag("//")),
            value(Operator::divide as OpCons, tag("/")),
        )),
        prefixed
    )(input)
}

fn sum<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    lbinop(
        alt((
            value(Operator::add as OpCons, tag("+")),
            value(Operator::subtract as OpCons, tag("-")),
        )),
        product,
    )(input)
}

fn inequality<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    lbinop(
        alt((
            value(Operator::less_equal as OpCons, tag("<=")),
            value(Operator::greater_equal as OpCons, tag(">=")),
            value(Operator::less as OpCons, tag("<")),
            value(Operator::greater as OpCons, tag(">")),
        )),
        sum,
    )(input)
}

fn equality<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    lbinop(
        alt((
            value(Operator::equal as OpCons, tag("==")),
            value(Operator::not_equal as OpCons, tag("!=")),
        )),
        inequality,
    )(input)
}

fn conjunction<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    lbinop(
        alt((
            value(Operator::and as OpCons, tag("and")),
        )),
        equality,
    )(input)
}

fn disjunction<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    lbinop(
        alt((
            value(Operator::or as OpCons, tag("or")),
        )),
        conjunction,
    )(input)
}

fn ident_binding<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Binding>, E> {
    postpad(alt((
        map(
            positioned(identifier),
            |out| out.map(Binding::Identifier),
        ),
    )))(input)
}

fn list_binding_element<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<ListBindingElement>, E> {
    positioned(alt((
        map(
            preceded(tag("..."), opt(identifier)),
            |ident| ident.map(ListBindingElement::SlurpTo).unwrap_or(ListBindingElement::Slurp),
        ),
        map(
            tuple((binding, opt(preceded(postpad(char('=')), expression)))),
            |(b, e)| ListBindingElement::Binding { binding: b, default: e },
        ),
    )))(input)
}

fn list_binding<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, ListBinding, E> {
    map(
        terminated(
            separated_list0(
                postpad(char(',')),
                list_binding_element,
            ),
            opt(postpad(char(','))),
        ),
        |x| ListBinding(x),
    )(input)
}

fn map_binding_element<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<MapBindingElement>, E> {
    positioned(alt((
        map(
            preceded(tag("..."), identifier),
            |i| MapBindingElement::SlurpTo(i),
        ),
        map(
            tuple((
                alt((
                    map(
                        tuple((
                            postpad(map_identifier),
                            preceded(
                                postpad(tag("as")),
                                binding,
                            ),
                        )),
                        |(name, binding)| (name, Some(binding)),
                    ),
                    map(
                        postpad(map_identifier),
                        |name| (name, None),
                    ),
                )),
                opt(
                    preceded(
                        postpad(char('=')),
                        expression,
                    ),
                ),
            )),
            |((name, binding), default)| {
                match binding {
                    None => MapBindingElement::Binding {
                        key: name,
                        binding: Binding::Identifier(name).tag(&name),
                        default,
                    },
                    Some(binding) => MapBindingElement::Binding {
                        key: name,
                        binding,
                        default,
                    },
                }
            },
        ),
    )))(input)
}

fn map_binding<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, MapBinding, E> {
    map(
        terminated(
            separated_list0(
                postpad(char(',')),
                map_binding_element,
            ),
            opt(postpad(char(','))),
        ),
        |x| MapBinding(x),
    )(input)
}

fn binding<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Binding>, E> {
    alt((
        ident_binding,
        postpad(positioned(map(positioned(delimited(postpad(char('[')), list_binding, char(']'))), Binding::List))),
        postpad(positioned(map(positioned(delimited(postpad(char('{')), map_binding, char('}'))), Binding::Map))),
    ))(input)
}

fn function<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
        tuple((
            delimited(
                postpad(char('(')),
                tuple((
                    list_binding,
                    opt(
                        preceded(
                            postpad(char(';')),
                            map_binding,
                        ),
                    ),
                )),
                postpad(char(')')),
            ),
            preceded(
                postpad(tag("=>")),
                expression,
            ),
        )),
        |((posargs, kwargs), expr)| Expr::Function {
            positional: posargs,
            keywords: kwargs,
            expression: Box::new(expr),
        },
    ))(input)
}

fn keyword_function<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
        tuple((
            delimited(
                postpad(char('{')),
                map_binding,
                postpad(char('}')),
            ),
            preceded(
                postpad(tag("=>")),
                expression,
            ),
        )),
        |(kwargs, expr)| Expr::Function {
            positional: ListBinding(vec![]),
            keywords: Some(kwargs),
            expression: Box::new(expr),
        },
    ))(input)
}

fn let_block<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
        tuple((
            many1(
                tuple((
                    preceded(
                        postpad(keyword("let")),
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
        |(bindings, expr)| Expr::Let { bindings, expression: Box::new(expr) },
    ))(input)
}

fn branch<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    positioned(map(
        tuple((
            preceded(
                postpad(keyword("if")),
                expression,
            ),
            preceded(
                postpad(keyword("then")),
                expression,
            ),
            preceded(
                postpad(keyword("else")),
                expression,
            ),
        )),
        |(condition, true_branch, false_branch)| Expr::Branch {
            condition: Box::new(condition),
            true_branch: Box::new(true_branch),
            false_branch: Box::new(false_branch),
        },
    ))(input)
}

fn composite<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    alt((
        let_block,
        branch,
        function,
        keyword_function,
    ))(input)
}

fn expression<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, Tagged<Expr>, E> {
    alt((
        composite,
        disjunction,
    ))(input)
}

fn import<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, TopLevel, E> {
    map(
        tuple((
            preceded(
                postpad(keyword("import")),
                postpad(preceded(
                    char('\"'),
                    terminated(raw_string, char('\"'))
                )),
            ),
            preceded(
                postpad(keyword("as")),
                postpad(binding),
            )
        )),
        |(path, binding)| TopLevel::Import(path, binding),
    )(input)
}

fn file<'a, E: CompleteError<'a>>(
    input: Span<'a>,
) -> IResult<Span<'a>, File, E> {
    map(
        tuple((
            many0(postpad(import)),
            preceded(multispace0, expression),
        )),
        |(statements, expression)| File { statements, expression },
    )(input)
}

pub fn parse(input: &str) -> Result<File, String> {
    let span = Span::new(input);
    file::<VerboseError<Span>>(span).map_or_else(
        |err| match err {
            Incomplete(_) => Err("incomplete input".to_string()),
            Error(e) | Failure(e) => Err(format!("{:#?}", e)),
        },
        |(remaining, node)| if remaining.len() > 0 {
            Err(format!("unconsumed input: {}", remaining))
        } else {
            node.validate()?;
            Ok(node)
        }
    )
}
