use std::{collections::HashMap, iter::FromIterator};
use nom::{IResult, Parser};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, alphanumeric1, char};
use nom::combinator::{map, recognize};
use nom::multi::many0;
use nom::sequence::{delimited, pair, preceded, tuple};

#[derive(Debug, Eq, PartialEq)]
pub struct BuildConditionValues {
    values: HashMap<String, String>,
}

impl BuildConditionValues {
    pub fn none() -> Self {
        Self {
            values: HashMap::new(),
        }
    }
}

impl<I: Iterator<Item = (String, String)>> From<I> for BuildConditionValues {
    fn from(source: I) -> Self {
        Self {
            values: HashMap::from_iter(source)
        }
    }
}

impl BuildConditionValues {
    fn lookup(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

// impl<
//     I: Iterator<Item = (K, V)>,
//     K: Eq + Into<String>,
//     V: Into<String>,
// > From<I> for BuildConditionValues {
//     fn from(source: I) -> Self {
//         fn stringise<K: Into<String>, V: Into<String>(tuple: &(K, V)) -> (String, String) {
//             (k.into(), v.into())
//         }
//         Self {
//             values: HashMap::from_iter(source.map(stringise))
//         }
//     }
// }

#[derive(Debug, Eq, PartialEq)]
pub enum BuildConditionExpression {
    None,
    Equal(BuildConditionTerm, BuildConditionTerm),
    Unequal(BuildConditionTerm, BuildConditionTerm),
}

#[derive(Debug, Eq, PartialEq)]
pub enum BuildConditionTerm {
    Literal(String),
    ValueRef(String),
}

impl BuildConditionExpression {
    pub fn parse(source: &Option<String>) -> anyhow::Result<Self> {
        match source {
            None => Ok(BuildConditionExpression::None),
            Some(rule_text) => Self::parse_rule(rule_text),
        }
    }

    fn parse_rule(rule_text: &str) -> anyhow::Result<Self> {
        match build_cond_expr(rule_text) {
            Ok((_, m)) => Ok(m),
            Err(e) => Err(anyhow::anyhow!("parse error {}", e)),
        }
    }

    pub fn should_build(&self, values: &BuildConditionValues) -> bool {
        match self {
            Self::None => true,
            Self::Equal(l, r) =>
                l.eval(values) == r.eval(values),
            Self::Unequal(l, r) =>
                l.eval(values) != r.eval(values),
        }
    }
}

impl BuildConditionTerm {
    fn eval(&self, values: &BuildConditionValues) -> Option<String> {
        match self {
            Self::Literal(s) => Some(s.clone()),
            Self::ValueRef(k) => values.lookup(k),
        }
    }
}

fn identifier(input: &str) -> IResult<&str, &str> {
    let (rest, m) = recognize(pair(
        alpha1,
        many0(alt((alphanumeric1, tag("_"))))
    ))(input)?;
    Ok((rest, m))
}

fn literal(input: &str) -> IResult<&str, BuildConditionTerm> {
    let mut literal_parser = delimited(
        char('\''),
        many0(alt((alphanumeric1, tag("_"), tag("-")))),
        char('\'')
    );
    let (rest, m) = literal_parser.parse(input)?;
    Ok((rest, BuildConditionTerm::Literal(m.join(""))))
}

fn term(input: &str) -> IResult<&str, BuildConditionTerm> {
    let value_ref = map(
        preceded(tag("$"), identifier),
        |m| BuildConditionTerm::ValueRef(m.to_owned())
    );
    let mut term_parser = alt((value_ref, literal));
    let (rest, m) = term_parser.parse(input)?;
    Ok((rest, m))
}

fn eq_op(input: &str) -> IResult<&str, EqOp> {
    let (rest, m) = recognize(alt((
        tag("=="),
        tag("!=")
    )))(input)?;
    Ok((rest, if m == "==" { EqOp::Equals } else { EqOp::DoesNotEqual }))
}

fn build_cond_expr(input: &str) -> IResult<&str, BuildConditionExpression> {
    // TODO: this is horrible
    let ws1 = many0(char(' '));
    let ws2 = many0(char(' '));
    let mut predicate = tuple((term, ws1, eq_op, ws2, term));
    let (rest, (left, _, op, _, right)) = predicate.parse(input)?;
    let expr = match op {
        EqOp::Equals => BuildConditionExpression::Equal(left, right),
        EqOp::DoesNotEqual => BuildConditionExpression::Unequal(left, right),
    };
    Ok((rest, expr))
}

enum EqOp { Equals, DoesNotEqual }

#[cfg(test)]
mod test {
    use super::*;

    fn build_kind(k: &str) -> impl Iterator<Item = (String, String)> {
        vec![("build_kind".to_owned(), k.to_owned())].into_iter()
    }

    #[test]
    fn test_expression_none_always_matches() {
        let expr = BuildConditionExpression::None;

        assert_eq!(true, expr.should_build(&BuildConditionValues::none()));
        assert_eq!(true, expr.should_build(&BuildConditionValues::from(build_kind("release"))));
        assert_eq!(true, expr.should_build(&BuildConditionValues::from(build_kind("debug"))));
    }

    #[test]
    fn test_equality_expression_matches_when_value_matches() {
        let expr = BuildConditionExpression::Equal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned())
        );

        assert_eq!(false, expr.should_build(&BuildConditionValues::none()));
        assert_eq!(true, expr.should_build(&BuildConditionValues::from(build_kind("release"))));
        assert_eq!(false, expr.should_build(&BuildConditionValues::from(build_kind("debug"))));
    }

    #[test]
    fn test_inequality_expression_matches_when_value_does_not() {
        let expr = BuildConditionExpression::Unequal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned())
        );

        assert_eq!(true, expr.should_build(&BuildConditionValues::none()));
        assert_eq!(false, expr.should_build(&BuildConditionValues::from(build_kind("release"))));
        assert_eq!(true, expr.should_build(&BuildConditionValues::from(build_kind("debug"))));
    }

    #[test]
    fn test_parsing_none_gives_expression_none() {
        assert_eq!(BuildConditionExpression::None, BuildConditionExpression::parse(&None).unwrap());
    }

    #[test]
    fn test_can_parse_equality_expressions() {
        let rule_text = Some("$build_kind == 'release'".to_owned());

        let expr = BuildConditionExpression::Equal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned())
        );

        assert_eq!(expr, BuildConditionExpression::parse(&rule_text).unwrap());
    }

    #[test]
    fn test_can_parse_inequality_expressions() {
        let rule_text = Some("$build_kind != 'release'".to_owned());

        let expr = BuildConditionExpression::Unequal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned())
        );

        assert_eq!(expr, BuildConditionExpression::parse(&rule_text).unwrap());
    }
}
