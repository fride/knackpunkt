use std::collections::{BTreeSet, BTreeMap};
extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;
use pest::iterators::Pair;
use bigdecimal::{BigDecimal, ToPrimitive};
use crate::edn::Value::TaggedElement;
use crate::edn::TaggedValue;

#[derive(Parser)]
#[grammar = "edn.pest"]
struct EDNParser;

pub mod edn {

    use std::collections::{BTreeMap, BTreeSet};
    use bigdecimal::BigDecimal;
    use std::panic::resume_unwind;

    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub struct TaggedValue {
        tag: String,
        value: Vec<Value>, // used as an indirection to not make the enum recursive. ;)
    }

    impl TaggedValue {
        pub fn new(tag: &str, value: Value) -> Self{
            Self {
                tag: tag.to_owned(),
                value: vec![value]
            }
        }
    }

    impl Into<TaggedValue> for (&str, Value) {
        fn into(self) -> TaggedValue {
            TaggedValue::new(self.0, self.1)
        }
    }

    impl ToString for TaggedValue {
        fn to_string(&self) -> String {
            format!("#{} {}", self.tag, &self.value[0].to_string())
        }
    }


    #[derive(Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
    pub enum Value {
        Nil,
        Keyword(Option<String>, String),
        String(String),
        Boolean(bool),
        Int(i64),
        BigDecimal(BigDecimal),
        TaggedElement(TaggedValue),
        List(Vec<Value>),
        Vec(Vec<Value>),
        Set(BTreeSet<Value>),
        Map(BTreeMap<Value, Value>)
    }

    impl ToString for Value {
        fn to_string(&self) -> String {
            match self {
                Value::Nil => "nil".to_owned(),
                Value::Keyword(Some(sns), n) => format!(":{}/{}", sns, n),
                Value::Keyword(_, n) =>  format!(":{}", n),
                Value::String(s) => format!("\"{}\"", s),
                Value::Boolean(true) => "true".to_owned(),
                Value::Boolean(false) => "false".to_owned(),
                Value::BigDecimal(n) => format!("{}", n),
                Value::TaggedElement(v) => v.to_string(),
                Value::List(elements) =>
                    format!("({})", elements
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<String>>()
                        .join(" "))
                ,
                Value::Vec(elements) =>
                    format!("[{}]", elements
                        .iter()
                        .map(|a| a.to_string())
                        .collect::<Vec<String>>()
                        .join(" "))
                ,
                Value::Set(values) => "".to_owned(),
                Value::Map(map) => {
                    let value_str = map.iter()
                        .map(|a| format!("{} {}", a.0.to_string(), a.1.to_string()))
                        .collect::<Vec<String>>()
                        .join(" ");
                    format!("{{{}}}", value_str)
                }
                Value::Int(i) => format!("{}", i)
            }
        }
    }
}

fn main() {
    use edn::Value;

    let str =  r###"
        { :example [{:person/name "Anna" :person/email "anna@example.com"}],
         : query {:crux.db/id #uuid "415c45c9-7cbe-4660-801b-dab9edc60c84", :value "baz"}
        }
    "###;

    let parse_result = EDNParser::parse(Rule::edn, str.trim()).unwrap().next().unwrap();

    fn parse_value(pair: Pair<Rule>) -> Value {
        match pair.as_rule() {
            Rule::map => {
                 let map_contents : Vec<(Value, Value)> = pair.into_inner().map(|pair| {
                     let mut inner_rules = pair.into_inner();
                     let key = parse_value(inner_rules.next()
                         .unwrap());
                     let value = parse_value(inner_rules.next().unwrap());
                     (key,value)
                    }
                 ).collect();
                let mut map = BTreeMap::new();
                for kv in map_contents.into_iter() {
                    map.insert(kv.0, kv.1);
                }
                Value::Map(map)
            }
            Rule::list => {
                Value::List(pair.into_inner()
                    .map(|value| {
                        parse_value(value)
                    })
                    .collect())
            }
            Rule::number => {
                let str_contents = pair.as_str();
                let num : BigDecimal = str_contents.parse().unwrap();
                if num.is_integer() {
                    num.to_i64()
                        .map(|int|Value::Int(int))
                        .unwrap_or(Value::BigDecimal(num))
                } else { Value::BigDecimal(num) }

            },
            Rule::vector => {
                Value::Vec(pair.into_inner()
                    .map(|value| {
                        parse_value(value)
                    })
                    .collect())
            }
            Rule::string => Value::String(pair.into_inner().next().unwrap().as_str().to_owned()),
            Rule::keyword => Value::Keyword(Option::None, pair.into_inner().next().unwrap().as_str().to_owned()),
            Rule::tagged_value => {
                let mut tag_and_value = pair.into_inner();
                let tag = tag_and_value.next().unwrap().as_str();
                let value = parse_value(tag_and_value.next().unwrap());
                Value::TaggedElement(TaggedValue::new(tag, value))
            }
            a => {
                println!("unhandled: {:?}", a);
                Value::Nil
            },
        }
    }
    let res = parse_value(parse_result);
    println!("{}", res.to_string());
}
