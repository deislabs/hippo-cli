use async_trait::async_trait;
use clap::{ArgMatches, Clap};
use std::path::PathBuf;

#[async_trait]
trait CommandRunner {
    fn run(&self) -> anyhow::Result<()>;
}

#[derive(Clap)]
#[clap(version = "1.0", author = "DeisLabs")]
pub(crate) struct NewHippo {
    #[clap(short, long)]
    destination: String,
    #[clap(short, long)]
    name: String,
}

#[async_trait]
impl CommandRunner for NewHippo {
    async fn run(&self) -> anyhow::Result<()> {
        // if dir is a directory, join with HIPPOFACTS. Otherwise, use it as a file name.
        let dest = if tokio::fs::metadata(self.destination).await?.is_dir() {
            PathBuf::from(self.destination).join("HIPPOFACTS")
        } else {
            PathBuf::from(self.destination)
        };

        let hippofacts = RawHippoFacts {
            bindle: hippofacts::BindleSpec {
                name: self.name,
                version: "0.1.0".to_owned(),
                description: None,
                authors: None,
            },
            annotations: None,
            handler: None,
            export: None,
        };
        let data = toml::to_vec(&hippofacts)?;
        tokio::fs::write(dest, data).await?;
        Ok(())
    }
}
