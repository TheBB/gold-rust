use std::rc::Rc;

use rug::Integer;

use crate::object::Object;


#[test]
fn to_string() {
    assert_eq!(Object::from(1).to_string(), "1");
    assert_eq!(Object::from(-1).to_string(), "-1");
    assert_eq!(
        Object::from(Integer::from_str_radix("9223372036854775808", 10).unwrap()).to_string(),
        "9223372036854775808",
    );

    assert_eq!(Object::from(1.2).to_string(), "1.2");
    assert_eq!(Object::from(1.0).to_string(), "1");

    assert_eq!(Object::from(-1.2).to_string(), "-1.2");
    assert_eq!(Object::from(-1.0).to_string(), "-1");

    assert_eq!(Object::from(true).to_string(), "true");
    assert_eq!(Object::from(false).to_string(), "false");
    assert_eq!(Object::Null.to_string(), "null");

    assert_eq!(Object::List(Rc::new(vec![])).to_string(), "[]");
    assert_eq!(Object::List(Rc::new(vec![
        Object::from(1),
        Object::from("alpha"),
    ])).to_string(), "[1, \"alpha\"]");

    assert_eq!(Object::from("alpha").to_string(), "\"alpha\"");
    assert_eq!(Object::from("\"alpha\\").to_string(), "\"\\\"alpha\\\\\"");
}


#[test]
fn format() {
    assert_eq!(Object::from("alpha").fmt(), Ok("alpha".to_string()));
    assert_eq!(Object::from("\"alpha\"").fmt(), Ok("\"alpha\"".to_string()));
    assert_eq!(Object::from("\"al\\pha\"").fmt(), Ok("\"al\\pha\"".to_string()));
}
