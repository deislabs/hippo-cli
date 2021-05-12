use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// type FeatureMap = BTreeMap<String, BTreeMap<String, String>>;

type AnnotationMap = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub handler: Option<Vec<Handler>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BindleSpec {
    pub name: String,
    pub version: String, // not semver::Version because this could be a template
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Handler {
    pub name: String,
    pub route: String,
    pub files: Option<Vec<String>>,
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
        name = "birds"
        version = "1.2.4"

        [[handler]]
        name = "penguin.wasm"
        route = "/birds/flightless"
        files = ["adelie.png", "rockhopper.png", "*.jpg"]

        [[handler]]
        name = "cassowary.wasm"
        route = "/birds/savage/rending"
        "#,
        )
        .expect("error parsing test TOML");

        assert_eq!("birds", &facts.bindle.name);
        assert_eq!(&None, &facts.annotations);

        let handlers = &facts.handler.expect("Expected handlers");

        assert_eq!(2, handlers.len());

        assert_eq!("penguin.wasm", &handlers[0].name);
        assert_eq!("/birds/flightless", &handlers[0].route);
        let files0 = handlers[0].files.as_ref().expect("Expected files");
        assert_eq!(3, files0.len());

        assert_eq!("cassowary.wasm", &handlers[1].name);
        assert_eq!("/birds/savage/rending", &handlers[1].route);
        assert_eq!(None, handlers[1].files);
    }
}
