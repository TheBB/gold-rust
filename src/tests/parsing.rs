use super::super::*;

#[test]
fn booleans_and_null() {
    assert_eq!(parse("true"), Ok(AstNode::Literal(Object::Boolean(true))));
    assert_eq!(parse("false"), Ok(AstNode::Literal(Object::Boolean(false))));
    assert_eq!(parse("null"), Ok(AstNode::Literal(Object::Null)));
}

#[test]
fn integers() {
    assert_eq!(parse("0"), Ok(AstNode::Literal(Object::Integer(0))));
    assert_eq!(parse("1"), Ok(AstNode::Literal(Object::Integer(1))));
    assert_eq!(parse("9223372036854775807"), Ok(AstNode::Literal(Object::Integer(9223372036854775807))));
    assert_eq!(
        parse("9223372036854775808"),
        Ok(AstNode::Literal(Object::BigInteger(BigInt::from_str_radix("9223372036854775808", 10).unwrap()))),
    );
}

#[test]
fn floats() {
    assert_eq!(parse("0.0"), Ok(AstNode::Literal(Object::Float(0.0))));
    assert_eq!(parse("0."), Ok(AstNode::Literal(Object::Float(0.0))));
    assert_eq!(parse(".0"), Ok(AstNode::Literal(Object::Float(0.0))));
    assert_eq!(parse("0e0"), Ok(AstNode::Literal(Object::Float(0.0))));
    assert_eq!(parse("0e1"), Ok(AstNode::Literal(Object::Float(0.0))));
    assert_eq!(parse("1."), Ok(AstNode::Literal(Object::Float(1.0))));
    assert_eq!(parse("1e+1"), Ok(AstNode::Literal(Object::Float(10.0))));
    assert_eq!(parse("1e1"), Ok(AstNode::Literal(Object::Float(10.0))));
    assert_eq!(parse("1e-1"), Ok(AstNode::Literal(Object::Float(0.1))));
}

#[test]
fn strings() {
    assert_eq!(parse("\"\""), Ok(AstNode::Literal(Object::String("".to_string()))));
    assert_eq!(parse("\"dingbob\""), Ok(AstNode::Literal(Object::String("dingbob".to_string()))));
    assert_eq!(parse("\"ding\\\"bob\""), Ok(AstNode::Literal(Object::String("ding\"bob".to_string()))));
    assert_eq!(parse("\"ding\\\\bob\""), Ok(AstNode::Literal(Object::String("ding\\bob".to_string()))));

    assert_eq!(
        parse("\"dingbob${a}\""),
        Ok(AstNode::String(vec![
            StringElement::Raw("dingbob".to_string()),
            StringElement::Interpolate(AstNode::Identifier("a".to_string())),
        ])),
    );

    assert_eq!(
        parse("\"dingbob${ a}\""),
        Ok(AstNode::String(vec![
            StringElement::Raw("dingbob".to_string()),
            StringElement::Interpolate(AstNode::Identifier("a".to_string())),
        ])),
    );
}

#[test]
fn identifiers() {
    assert_eq!(parse("dingbob"), Ok(AstNode::Identifier("dingbob".to_string())));
    assert_eq!(parse("lets"), Ok(AstNode::Identifier("lets".to_string())));
    assert_eq!(parse("not1"), Ok(AstNode::Identifier("not1".to_string())));
}

#[test]
fn lists() {
    assert_eq!(parse("[]"), Ok(AstNode::List(vec![])));
    assert_eq!(parse("[   ]"), Ok(AstNode::List(vec![])));

    assert_eq!(
        parse("[true]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Boolean(true))),
        ])),
    );

    assert_eq!(
        parse("[\"\"]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::String("".to_string()))),
        ])),
    );

    assert_eq!(
        parse("[1,]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Integer(1))),
        ])),
    );

    assert_eq!(
        parse("[  1   ,  ]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Integer(1))),
        ])),
    );

    assert_eq!(
        parse("[  1   ,2  ]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Integer(1))),
            ListElement::Singleton(AstNode::Literal(Object::Integer(2))),
        ])),
    );

    assert_eq!(
        parse("[  1   ,2  ,]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Integer(1))),
            ListElement::Singleton(AstNode::Literal(Object::Integer(2))),
        ])),
    );

    assert_eq!(
        parse("[1, false, 2.3, \"fable\", lel]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Integer(1))),
            ListElement::Singleton(AstNode::Literal(Object::Boolean(false))),
            ListElement::Singleton(AstNode::Literal(Object::Float(2.3))),
            ListElement::Singleton(AstNode::Literal(Object::String("fable".to_string()))),
            ListElement::Singleton(AstNode::Identifier("lel".to_string())),
        ])),
    );
}

#[test]
fn nested_lists() {
    assert_eq!(
        parse("[[]]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::List(vec![])),
        ])),
    );

    assert_eq!(
        parse("[1, [2]]"),
        Ok(AstNode::List(vec![
            ListElement::Singleton(AstNode::Literal(Object::Integer(1))),
            ListElement::Singleton(AstNode::List(vec![
                ListElement::Singleton(AstNode::Literal(Object::Integer(2))),
            ])),
        ])),
    );
}

#[test]
fn maps() {
    assert_eq!(parse("{}"), Ok(AstNode::Map(vec![])));
    assert_eq!(parse("{  }"), Ok(AstNode::Map(vec![])));

    assert_eq!(
        parse("{a = 1}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("a".to_string())),
                AstNode::Literal(Object::Integer(1)),
            ),
        ])),
    );

    assert_eq!(
        parse("{a = 1,}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("a".to_string())),
                AstNode::Literal(Object::Integer(1)),
            ),
        ])),
    );

    assert_eq!(
        parse("{  a=1,}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("a".to_string())),
                AstNode::Literal(Object::Integer(1)),
            ),
        ])),
    );

    assert_eq!(
        parse("{a = 1  ,b=2}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("a".to_string())),
                AstNode::Literal(Object::Integer(1)),
            ),
            MapElement::Singleton(
                AstNode::Literal(Object::String("b".to_string())),
                AstNode::Literal(Object::Integer(2)),
            ),
        ])),
    );

    assert_eq!(
        parse("{che9 = false}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("che9".to_string())),
                AstNode::Literal(Object::Boolean(false)),
            ),
        ])),
    );

    assert_eq!(
        parse("{fable = \"fable\"}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("fable".to_string())),
                AstNode::Literal(Object::String("fable".to_string())),
            ),
        ])),
    );

    assert_eq!(
        parse("{a = 1, b = true, c = 2.e1, d = \"hoho\", e = 1e1}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("a".to_string())),
                AstNode::Literal(Object::Integer(1)),
            ),
            MapElement::Singleton(
                AstNode::Literal(Object::String("b".to_string())),
                AstNode::Literal(Object::Boolean(true)),
            ),
            MapElement::Singleton(
                AstNode::Literal(Object::String("c".to_string())),
                AstNode::Literal(Object::Float(20.0)),
            ),
            MapElement::Singleton(
                AstNode::Literal(Object::String("d".to_string())),
                AstNode::Literal(Object::String("hoho".to_string())),
            ),
            MapElement::Singleton(
                AstNode::Literal(Object::String("e".to_string())),
                AstNode::Literal(Object::Float(10.0)),
            ),
        ])),
    );

    assert_eq!(
        parse("{ident-with-hyphen = 1}"),
        Ok(AstNode::Map(vec![
            MapElement::Singleton(
                AstNode::Literal(Object::String("ident-with-hyphen".to_string())),
                AstNode::Literal(Object::Integer(1)),
            )
        ])),
    );
}

#[test]
fn let_blocks() {
    assert_eq!(
        parse("let a = \"b\" in 1"),
        Ok(AstNode::Let(
            vec![
                (Binding::Identifier("a".to_string()), AstNode::Literal(Object::String("b".to_string()))),
            ],
            Box::new(AstNode::Literal(Object::Integer(1))),
        )),
    );

    assert_eq!(
        parse("let a = 1 let b = 2 in a"),
        Ok(AstNode::Let(
            vec![
                (Binding::Identifier("a".to_string()), AstNode::Literal(Object::Integer(1))),
                (Binding::Identifier("b".to_string()), AstNode::Literal(Object::Integer(2))),
            ],
            Box::new(AstNode::Identifier("a".to_string())),
        )),
    );
}

#[test]
fn indexing() {
    assert_eq!(
        parse("a.b"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Identifier("a".to_string())),
            Operator::Index(Box::new(AstNode::Literal(Object::String("b".to_string())))),
        )),
    );

    assert_eq!(
        parse("a[b]"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Identifier("a".to_string())),
            Operator::Index(Box::new(AstNode::Identifier("b".to_string()))),
        )),
    );

    assert_eq!(
        parse("a.b.c"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Operator(
                Box::new(AstNode::Identifier("a".to_string())),
                Operator::Index(Box::new(AstNode::Literal(Object::String("b".to_string())))),
            )),
            Operator::Index(Box::new(AstNode::Literal(Object::String("c".to_string())))),
        )),
    );

    assert_eq!(
        parse("a[b].c"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Operator(
                Box::new(AstNode::Identifier("a".to_string())),
                Operator::Index(Box::new(AstNode::Identifier("b".to_string()))),
            )),
            Operator::Index(Box::new(AstNode::Literal(Object::String("c".to_string())))),
        )),
    );

    assert_eq!(
        parse("a.b[c]"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Operator(
                Box::new(AstNode::Identifier("a".to_string())),
                Operator::Index(Box::new(AstNode::Literal(Object::String("b".to_string())))),
            )),
            Operator::Index(Box::new(AstNode::Identifier("c".to_string()))),
        )),
    );

    assert_eq!(
        parse("a[b][c]"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Operator(
                Box::new(AstNode::Identifier("a".to_string())),
                Operator::Index(Box::new(AstNode::Identifier("b".to_string()))),
            )),
            Operator::Index(Box::new(AstNode::Identifier("c".to_string()))),
        )),
    );
}

#[test]
fn unary_operators() {
    assert_eq!(
        parse("-1"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Literal(Object::Integer(1))),
            Operator::ArithmeticalNegate,
        )),
    );

    assert_eq!(
        parse("- not 1"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Operator(
                Box::new(AstNode::Literal(Object::Integer(1))),
                Operator::LogicalNegate,
            )),
            Operator::ArithmeticalNegate,
        )),
    );

    assert_eq!(
        parse("not -1"),
        Ok(AstNode::Operator(
            Box::new(AstNode::Operator(
                Box::new(AstNode::Literal(Object::Integer(1))),
                Operator::ArithmeticalNegate,
            )),
            Operator::LogicalNegate,
        )),
    );
}
