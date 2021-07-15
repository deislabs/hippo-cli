use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, convert::TryFrom};

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
    pub name: Option<String>,
    pub external: Option<RawExternalRef>,
    pub route: String,
    pub files: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawExternalRef {
    pub bindle_id: String,
    pub handler_id: String,
}

pub struct HippoFacts {
    pub bindle: BindleSpec,
    pub annotations: Option<AnnotationMap>,
    pub handler: Vec<Handler>,
}

pub struct Handler {
    pub handler_module: HandlerModule,
    pub route: String,
    pub files: Option<Vec<String>>,
}

pub enum HandlerModule {
    File(String),
    External(ExternalRef),
}

pub struct ExternalRef {
    pub bindle_id: bindle::Id,
    pub handler_id: String,
}

impl HippoFacts {
    pub fn read_from(source: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        // Immediate-call closure lets us use the try operator
        let read_result = (|| {
            let content = std::fs::read_to_string(&source)?;
            let spec = toml::from_str::<RawHippoFacts>(&content)?;
            Self::try_from(&spec)
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

impl TryFrom<&RawHippoFacts> for HippoFacts {
    type Error = anyhow::Error;

    fn try_from(raw: &RawHippoFacts) -> anyhow::Result<Self> {
        let handler_vec = raw.handler.as_ref().ok_or_else(no_handlers)?;
        let handler: anyhow::Result<Vec<_>> = handler_vec.iter().map(Handler::try_from).collect();
        Ok(Self {
            bindle: raw.bindle.clone(),
            annotations: raw.annotations.clone(),
            handler: handler?,
        })
    }
}

impl TryFrom<&RawHandler> for Handler {
    type Error = anyhow::Error;

    fn try_from(raw: &RawHandler) -> anyhow::Result<Handler> {
        let handler_module = match (&raw.name, &raw.external) {
            (Some(name), None) => Ok(HandlerModule::File(name.clone())),
            (None, Some(external_ref)) => Ok(HandlerModule::External(ExternalRef::try_from(external_ref)?)),
            _ => Err(anyhow::anyhow!("Route '{}' must specify exactly one of 'name' and 'external'", raw.route)),
        }?;
        Ok(Self {
            handler_module,
            route: raw.route.clone(),
            files: raw.files.clone(),
        })
    }
}

impl TryFrom<&RawExternalRef> for ExternalRef {
    type Error = anyhow::Error;

    fn try_from(raw: &RawExternalRef) -> anyhow::Result<ExternalRef> {
        let bindle_id = bindle::Id::try_from(&raw.bindle_id)?;
        Ok(Self {
            bindle_id,
            handler_id: raw.handler_id.clone(),
        })
    }
}

fn no_handlers() -> anyhow::Error {
    anyhow::anyhow!("No handlers defined in artifact spec")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_can_read_hippo_facts() {
        let raw: RawHippoFacts = toml::from_str(
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
        let facts = HippoFacts::try_from(&raw).expect("error parsing raw to HF");

        assert_eq!("birds", &facts.bindle.name);
        assert_eq!(&None, &facts.annotations);

        let handlers = &facts.handler;

        assert_eq!(2, handlers.len());

        if let HandlerModule::File(name) = &handlers[0].handler_module {
            assert_eq!("penguin.wasm", name);
        } else {
            assert!(false, "handler 0 should have been File");
        }
        assert_eq!("/birds/flightless", &handlers[0].route);
        let files0 = handlers[0].files.as_ref().expect("Expected files");
        assert_eq!(3, files0.len());

        if let HandlerModule::File(name) = &handlers[1].handler_module {
            assert_eq!("cassowary.wasm", name);
        } else {
            assert!(false, "handler 1 should have been File");
        }
        assert_eq!("/birds/savage/rending", &handlers[1].route);
        assert_eq!(None, handlers[1].files);
    }

    #[test]
    fn test_parse_externals() {
        let facts = HippoFacts::read_from("./testdata/external1/HIPPOFACTS").expect("error reading facts file");

        assert_eq!("toastbattle", &facts.bindle.name);

        let handlers = &facts.handler;

        assert_eq!(2, handlers.len());

        if let HandlerModule::External(ext) = &handlers[1].handler_module {
            assert_eq!("deislabs/fileserver", ext.bindle_id.name());
            assert_eq!("1.0.3", ext.bindle_id.version_string());
            assert_eq!("static", ext.handler_id);
        } else {
            assert!(false, "handler 1 should have been External");
        }
    }
}
