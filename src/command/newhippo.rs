use crate::hippofacts::{BindleSpec, RawHandler, RawHippoFacts};
use async_trait::async_trait;
use clap::{App, Arg, ArgMatches};
use std::path::PathBuf;

pub(crate) const CMD_NEW_HIPPO: &str = "hippofacts";
pub(crate) const CMD_NEW: &str = "new";

/// The top-level subcommand for `hippo new`
pub(crate) struct NewSubcommand;

#[async_trait]
impl super::CommandRunner for NewSubcommand {
    fn app<'a>() -> App<'a> {
        App::new(CMD_NEW)
            .about("Create project-specific files")
            .subcommand(NewHippofacts::app())
    }

    async fn run(&self, matches: &ArgMatches) -> anyhow::Result<()> {
        match matches.subcommand() {
            Some((CMD_NEW_HIPPO, args)) => {
                let cmd = NewHippofacts;
                cmd.run(args).await
            }
            Some((cmd, _)) => anyhow::bail!("Unknown subcommand: {}", cmd),
            None => anyhow::bail!(
            "Use one of the subcommands, such as 'hippo new hippofacts'. Try 'hippo new --help'."
        ),
        }
    }
}

/// The subcommand for `hippo new hippofacts`
pub(crate) struct NewHippofacts;

#[async_trait]
impl super::CommandRunner for NewHippofacts {
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
        .arg(
            Arg::new("author")
            .multiple_occurrences(true)
            .value_name("AUTHOR")
            .short('a')
            .long("author")
            .about("The name(s) and email(s) of the author(s): 'First Last <user@example.com>'")
        )
        .arg(
            Arg::new("module_name")
            .takes_value(true)
            .value_name("MODULE.WASM")
            .short('m')
            .long("module")
            .about("The path to the Wasm module. Example: 'bin/main.wasm'")
        )
    }

    async fn run(&self, opts: &ArgMatches) -> anyhow::Result<()> {
        let destination = opts.value_of("destination_dir").unwrap_or(".").to_string();
        let name = opts.value_of("name").unwrap().to_string(); // TODO: I think required(true) means this is safe.
        let author = opts.values_of_lossy("author");
        let modname = opts.value_of("module_name").unwrap_or("main.wasm");
        let handler = RawHandler {
            name: Some(modname.to_owned()),
            entrypoint: None,
            route: "/".to_owned(),
            files: None,
            external: None,
        };

        // if dir is a directory, join with HIPPOFACTS. Otherwise, use it as a file name.
        let md = tokio::fs::metadata(&destination).await?;
        let dest = if md.is_dir() {
            PathBuf::from(&destination).join("HIPPOFACTS")
        } else {
            PathBuf::from(&destination)
        };

        // Don't overwrite an existing HIPPOFACTS
        match tokio::fs::metadata(&dest).await {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
            Ok(_) => {
                anyhow::bail!("Cowardly refusing to overwrite file. Remove HIPPOFACTS first.")
            }
        }?;

        let hippofacts = RawHippoFacts {
            bindle: BindleSpec {
                name,
                version: "0.1.0".to_owned(),
                description: None,
                authors: author,
            },
            annotations: None,
            export: None,
            handler: Some(vec![handler]),
        };

        let data = toml::to_vec(&hippofacts)?;
        tokio::fs::write(dest, data).await?;
        Ok(())
    }
}
