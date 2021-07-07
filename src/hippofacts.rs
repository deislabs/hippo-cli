use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// type FeatureMap = BTreeMap<String, BTreeMap<String, String>>;

type AnnotationMap = BTreeMap<String, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawHippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub handler: Option<Vec<RawHandler>>,
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
struct RawHandler {
    pub name: String,
    pub route: String,
    pub files: Option<Vec<String>>,
}

pub struct HippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub handler: Option<Vec<Handler>>,
}

pub struct Handler {
    pub handler_module: HandlerModule,
    pub route: String,
    pub files: Option<Vec<String>>,
}

pub enum HandlerModule {
    File(String),
}

impl HippoFacts {
    pub fn read_from(source: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        // Immediate-call closure lets us use the try operator
        let read_result = (|| {
            let content = std::fs::read_to_string(&source)?;
            let spec = toml::from_str::<RawHippoFacts>(&content)?;
            Ok(Self::from(&spec))
        })();
        read_result.map_err(|e: anyhow::Error| {
            anyhow::anyhow!(
                "Error parsing {} as a Hippo artifacts file: {}",
                source.as_ref().to_string_lossy(),
                e
            )
        })
    }
}

impl From<&RawHippoFacts> for HippoFacts {
    fn from(raw: &RawHippoFacts) -> Self {
        Self {
            bindle: raw.bindle.clone(),
            annotations: raw.annotations.clone(),
            handler: raw.handler.as_ref().map(|v| v.iter().map(Handler::from).collect()),
        }
    }
}

impl From<&RawHandler> for Handler {
    fn from(raw: &RawHandler) -> Handler {
        Self {
            handler_module: HandlerModule::File(raw.name.clone()),
            route: raw.route.clone(),
            files: raw.files.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_can_read_hippo_facts() {
        let facts: RawHippoFacts = toml::from_str(
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
