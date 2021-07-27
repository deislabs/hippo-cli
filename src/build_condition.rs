pub struct BuildConditionValues {
    //
}

impl BuildConditionValues {
    pub fn none() -> Self {
        Self {}
    }
}

pub enum BuildConditionExpression {
    None,
}

impl BuildConditionExpression {
    pub fn should_build(&self, _values: &BuildConditionValues) -> bool {
        true
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     // impl BuildConditionValues {
//     //     pub fn none() -> Self {
//     //         Self {}
//     //     }
//     // }
// }
