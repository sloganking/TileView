use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(version)]
pub struct Args {
    /// Whether to show stats in the top left
    #[clap(long)]
    pub stats: bool,

    /// Whether to show borders around tiles
    #[clap(long)]
    pub tiles: bool,

    /// The path to the image or tiles to render
    pub image_path: PathBuf,
}
