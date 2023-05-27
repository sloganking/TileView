use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(version)]
pub struct Args {
    pub image_path: PathBuf,
}
