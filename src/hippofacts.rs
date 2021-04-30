use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// type FeatureMap = BTreeMap<String, BTreeMap<String, String>>;

type AnnotationMap = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub files: std::collections::BTreeMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BindleSpec {
    pub name: String,
    pub version: String, // not semver::Version because this could be a template
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_can_read_hippo_facts() {
        let facts: HippoFacts = toml::from_str(
            r#"
        # HIPPO FACT: the North American house hippo is found across Canada and the Eastern US
        [bindle]
        name = "weather"
        version = "1.2.4"

        [files]
        server = [
            "*.wasm",
            "gadget.jsx"
        ]
        client = [
            "images/*",
            "scripts/*.js",
            "css/*.css"
        ]
        "#,
        )
        .expect("error parsing test TOML");

        assert_eq!("weather", &facts.bindle.name);
        assert_eq!(&None, &facts.annotations);
        assert_eq!(
            2,
            facts.files.get("server").expect("no server section").len()
        );
        assert_eq!(
            3,
            facts.files.get("client").expect("no client section").len()
        );
    }
}
