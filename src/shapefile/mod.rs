use std::{error::Error, path::Path};

use log::info;
use std::path::PathBuf;

use crate::{config::Config, io::fs::FileSystem};

mod canvas;
mod mapping;
mod render;

pub use render::render;

/// Unzips the shape files and renders them to a canvas.
pub fn unzip_and_render(
    fs: &impl FileSystem,
    config: &Config,
    tmpfolder: &Path,
    filenames: &[String],
) -> Result<(), Box<dyn Error>> {
    for zip_name in filenames.iter() {
        info!("Opening zip file {zip_name}");
        let file = fs.open(zip_name).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        info!(
            "Extracting {:?} MB from {zip_name}",
            archive.decompressed_size().map(|s| s / 1024 / 1024)
        );
        archive.extract(tmpfolder).unwrap();
    }

    render::render(fs, config, tmpfolder, false).unwrap();

    Ok(())
}

/// Unzips the shape files to specific folder
pub fn unzip_shapefiles(fs: &impl FileSystem, filenames: &[String]) -> Result<(), Box<dyn Error>> {
    for zip_name in filenames.iter() {
        info!("Opening zip file {zip_name}");
        let file = fs.open(zip_name).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        info!(
            "Extracting {:?} MB from {zip_name}",
            archive.decompressed_size().map(|s| s / 1024 / 1024)
        );
        let tmpfolder = PathBuf::from("temp_shapefiles".to_string());
        archive.extract(tmpfolder).unwrap();
    }
    Ok(())
}
