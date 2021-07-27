use std::{collections::HashMap, iter::FromIterator};

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

pub enum BuildConditionExpression {
    None,
    Equal(BuildConditionTerm, BuildConditionTerm),
    Unequal(BuildConditionTerm, BuildConditionTerm),
}

pub enum BuildConditionTerm {
    Literal(String),
    ValueRef(String),
}

impl BuildConditionExpression {
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

#[cfg(test)]
mod test {
    use super::*;

    // impl BuildConditionValues {
    //     pub fn none() -> Self {
    //         Self {}
    //     }
    // }

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
}
