use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(version)]
pub struct Args {
    /// Whether to show borders around tiles
    #[clap(long)]
    pub tiles: bool,

    /// The path to the image or tiles to render
    pub image_path: PathBuf,
}
