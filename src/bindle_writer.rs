use std::path::{Path, PathBuf};

use crate::invoice::Invoice;

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
        todo!("oh no")
    }
}
