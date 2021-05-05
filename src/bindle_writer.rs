use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::invoice::{Invoice, Parcel};

pub struct BindleWriter {
    source_base_path: PathBuf,
    dest_base_path: PathBuf,
}

impl BindleWriter {
    pub fn new(source_base_path: impl AsRef<Path>, dest_base_path: impl AsRef<Path>) -> Self {
        Self {
            source_base_path: source_base_path.as_ref().to_path_buf(),
            dest_base_path: dest_base_path.as_ref().to_path_buf(),
        }
    }

    pub async fn write(&self, invoice: &Invoice) -> anyhow::Result<()> {
        // This is very similar to bindle::StandaloneWrite::write but... not quite the same
        let bindle_id_hash = sha(&invoice.bindle.name, &invoice.bindle.version);
        let bindle_dir = self.dest_base_path.join(bindle_id_hash);
        let parcels_dir = bindle_dir.join("parcels");
        async_std::fs::create_dir_all(&parcels_dir).await?;

        self.write_invoice_file(invoice, &bindle_dir).await?;
        self.write_parcel_files(invoice, &parcels_dir).await?;
        Ok(())
    }

    async fn write_invoice_file(&self, invoice: &Invoice, bindle_dir: &PathBuf) -> anyhow::Result<()> {
        let invoice_text = toml::to_string_pretty(&invoice)?;
        let invoice_file = bindle_dir.join("invoice.toml");
        async_std::fs::write(&invoice_file, &invoice_text).await?;
        Ok(())
    }

    async fn write_parcel_files(&self, invoice: &Invoice, parcels_dir: &PathBuf) -> anyhow::Result<()> {
        let parcels = match &invoice.parcel {
            Some(p) => p,
            None => return Ok(()),
        };

        let parcel_writes = parcels.iter().map(|parcel| self.write_one_parcel(parcels_dir, &parcel));
        futures::future::join_all(parcel_writes).await.into_iter().collect::<anyhow::Result<Vec<_>>>()?;
        Ok(())
    }

    async fn write_one_parcel(&self, parcels_dir: &PathBuf, parcel: &Parcel) -> anyhow::Result<()> {
        let source_file = self.source_base_path.join(&parcel.label.name);
        let hash = &parcel.label.sha256;
        let dest_file = parcels_dir.join(format!("{}.dat", hash));
        async_std::fs::copy(&source_file, &dest_file).await?;
        Ok(())
    }
}

fn sha(name: &str, version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name);
    hasher.update("/");
    hasher.update(version);
    let result = hasher.finalize();
    format!("{:x}", result)
}
