use crate::novatek_gps;
use std::io;
use std::path::PathBuf;

#[derive(Debug, err_derive::Error)]
pub enum Error {
    #[error(display = "IO error")]
    Io(#[error(source)] io::Error),

    #[error(display = "MP4 error")]
    Mp4(#[error(source)] mp4::Error),

    #[error(
        display = "The output file {:?} already exists, use --force to overwrite",
        _0
    )]
    OutputFileExists(PathBuf),

    #[error(display = "The input path {:?} is not a file", _0)]
    PathNotFile(PathBuf),

    #[error(display = "GPS error")]
    Gps(#[error(source)] novatek_gps::Error),

    #[error(display = "GPX error")]
    Gpx(#[error(source)] gpx::errors::Error),

    #[error(display = "Glob pattern error")]
    Glob(#[error(source)] glob::PatternError),
}
