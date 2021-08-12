use crate::hippofacts::{BindleSpec, RawHippoFacts};
use async_trait::async_trait;
use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

pub(crate) const CMD_NEW_HIPPO: &str = "new";

pub(crate) struct NewHippo;

#[async_trait]
impl super::CommandRunner for NewHippo {
    // ^^ World's most boring super hero.

    fn app<'a>() -> App<'a> {
        App::new(CMD_NEW_HIPPO)
        .about("Creates a new hipppo project with a HIPPOFACTS file.")
        .arg(
            Arg::new("destination_dir")
            .takes_value(true)
            .value_name("FILE_OR_DIR")
            .short('d')
            .long("destination")
            .about("The directory or file into which the HIPPOFACTS file should be written. If this is a directory, a HIPPOFACTS file will be written to the directory. Otherwise, a new file will be created with the given name.")
        )
        .arg(
            Arg::new("name")
            .value_name("NAME")
            .about("The name of the new application")
            .index(1)
            .required(true)
        )
    }
    async fn run(&self, opts: &ArgMatches) -> anyhow::Result<()> {
        let destination = opts.value_of("destination_dir").unwrap_or(".").to_string();
        let name = opts.value_of("name").unwrap().to_string(); // TODO: I think required(true) means this is safe.

        // if dir is a directory, join with HIPPOFACTS. Otherwise, use it as a file name.
        let dest = if tokio::fs::metadata(&destination).await?.is_dir() {
            PathBuf::from(&destination).join("HIPPOFACTS")
        } else {
            PathBuf::from(&destination)
        };

        let hippofacts = RawHippoFacts {
            bindle: BindleSpec {
                name,
                version: "0.1.0".to_owned(),
                description: None,
                authors: None,
            },
            annotations: None,
            export: None,
            handler: None,
        };

        let data = toml::to_vec(&hippofacts)?;
        tokio::fs::write(dest, data).await?;
        Ok(())
    }
}
