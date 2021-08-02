use itertools::Itertools;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alpha1, alphanumeric1, char};
use nom::combinator::recognize;
use nom::multi::many0;
use nom::sequence::{delimited, pair, preceded, tuple};
use nom::Parser as NomParser; // Name doesn't matter: we only want it for its methods
use std::collections::HashMap;

type Span<'a> = nom_locate::LocatedSpan<&'a str>;

trait Parser<'a, T>: nom::Parser<Span<'a>, T, nom::error::Error<Span<'a>>> {}
impl<'a, T, P> Parser<'a, T> for P where P: nom::Parser<Span<'a>, T, nom::error::Error<Span<'a>>> {}

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

    fn lookup(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

impl<I: Iterator<Item = (String, String)>> From<I> for BuildConditionValues {
    fn from(source: I) -> Self {
        Self {
            values: source.collect(),
        }
    }
}

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
        let mut parser = build_cond_expr();
        match parser.parse(Span::new(rule_text)) {
            Ok((_, m)) => Ok(m),
            Err(e) => Err(Self::describe_parse_error(rule_text, e)),
        }
    }

    fn describe_parse_error(
        parse_text: &str,
        error: nom::Err<nom::error::Error<Span>>,
    ) -> anyhow::Error {
        let message = match &error {
            nom::Err::Incomplete(_) => "unexpected end of condition".to_owned(),
            nom::Err::Failure(e) => Self::error_text_of(e),
            nom::Err::Error(e) => Self::error_text_of(e),
        };
        let offset = match &error {
            nom::Err::Incomplete(_) => None,
            nom::Err::Failure(e) => Some(e.input.location_offset()),
            nom::Err::Error(e) => Some(e.input.location_offset()),
        };
        let err_line = format!(
            r#"Invalid build condition "{}". Typical format is: "$name ==/!= 'value'"; problem was {}"#,
            parse_text, message
        );
        let diagnostics_lines = match offset {
            None => vec![],
            Some(offset) => vec![
                format!("    {}", parse_text),
                format!("    {}^-- here", " ".repeat(offset)),
            ],
        };
        let all_lines = vec![err_line]
            .iter()
            .chain(diagnostics_lines.iter())
            .join("\n");
        anyhow::Error::msg(all_lines)
    }

    fn error_text_of(e: &nom::error::Error<Span>) -> String {
        match start_text_of(e.input) {
            "" => "unexpected end of condition".to_owned(),
            s => format!(r#"unexpected text "{}""#, s),
        }
    }

    pub fn should_build(&self, values: &BuildConditionValues) -> bool {
        match self {
            Self::None => true,
            Self::Equal(l, r) => l.eval(values) == r.eval(values),
            Self::Unequal(l, r) => l.eval(values) != r.eval(values),
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

fn identifier<'a>() -> impl Parser<'a, Span<'a>> {
    recognize(pair(alpha1, many0(alt((alphanumeric1, tag("_"))))))
}

fn value<'a>() -> impl Parser<'a, Span<'a>> {
    recognize(many0(alt((alphanumeric1, tag("_"), tag("-"), tag(".")))))
}

fn value_ref<'a>() -> impl Parser<'a, BuildConditionTerm> {
    preceded(tag("$"), identifier()).map(|m| BuildConditionTerm::ValueRef((*m).to_owned()))
}

fn literal<'a>() -> impl Parser<'a, BuildConditionTerm> {
    delimited(char('\''), value(), char('\'')).map(|m| BuildConditionTerm::Literal((*m).to_owned()))
}

fn term<'a>() -> impl Parser<'a, BuildConditionTerm> {
    alt((value_ref(), literal()))
}

fn ws<'a>() -> impl Parser<'a, ()> {
    many0(char(' ')).map(|_| ())
}

fn binary_op<'a>(
) -> impl Parser<'a, fn(BuildConditionTerm, BuildConditionTerm) -> BuildConditionExpression> {
    alt((tag("=="), tag("!="))).map(|m: Span| parse_binary_op(*m))
}

fn parse_binary_op(
    text: &str,
) -> fn(BuildConditionTerm, BuildConditionTerm) -> BuildConditionExpression {
    if text == "==" {
        BuildConditionExpression::Equal
    } else {
        BuildConditionExpression::Unequal
    }
}

fn build_cond_expr<'a>() -> impl Parser<'a, BuildConditionExpression> {
    tuple((term(), ws(), binary_op(), ws(), term())).map(|(left, _, op, _, right)| op(left, right))
}

fn start_text_of(text: Span) -> &str {
    text.split(' ').next().unwrap_or("")
}

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
        assert_eq!(
            true,
            expr.should_build(&BuildConditionValues::from(build_kind("release")))
        );
        assert_eq!(
            true,
            expr.should_build(&BuildConditionValues::from(build_kind("debug")))
        );
    }

    #[test]
    fn test_equality_expression_matches_when_value_matches() {
        let expr = BuildConditionExpression::Equal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned()),
        );

        assert_eq!(false, expr.should_build(&BuildConditionValues::none()));
        assert_eq!(
            true,
            expr.should_build(&BuildConditionValues::from(build_kind("release")))
        );
        assert_eq!(
            false,
            expr.should_build(&BuildConditionValues::from(build_kind("debug")))
        );
    }

    #[test]
    fn test_inequality_expression_matches_when_value_does_not() {
        let expr = BuildConditionExpression::Unequal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned()),
        );

        assert_eq!(true, expr.should_build(&BuildConditionValues::none()));
        assert_eq!(
            false,
            expr.should_build(&BuildConditionValues::from(build_kind("release")))
        );
        assert_eq!(
            true,
            expr.should_build(&BuildConditionValues::from(build_kind("debug")))
        );
    }

    #[test]
    fn test_parsing_none_gives_expression_none() {
        assert_eq!(
            BuildConditionExpression::None,
            BuildConditionExpression::parse(&None).unwrap()
        );
    }

    #[test]
    fn test_can_parse_equality_expressions() {
        let rule_text = Some("$build_kind == 'release'".to_owned());

        let expr = BuildConditionExpression::Equal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned()),
        );

        assert_eq!(expr, BuildConditionExpression::parse(&rule_text).unwrap());
    }

    #[test]
    fn test_can_parse_inequality_expressions() {
        let rule_text = Some("$build_kind != 'release'".to_owned());

        let expr = BuildConditionExpression::Unequal(
            BuildConditionTerm::ValueRef("build_kind".to_owned()),
            BuildConditionTerm::Literal("release".to_owned()),
        );

        assert_eq!(expr, BuildConditionExpression::parse(&rule_text).unwrap());
    }
}
