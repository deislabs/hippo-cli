use std::path::Path;

use bindle::standalone::StandaloneRead;

pub async fn push_all(path: impl AsRef<Path>, bindle_id: &bindle::Id, base_url: &str) -> anyhow::Result<()> {
    let reader = StandaloneRead::new(&path, bindle_id).await?;
    let client = bindle::client::Client::new(base_url)?;
    reader.push(&client).await?;
    Ok(())
}
