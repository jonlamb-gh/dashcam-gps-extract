use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::{clap, StructOpt};

const ABOUT: &str = r#"Convert Novatek GPS data embedded into mp4 file(s) into gpx.

Examples:
    dashcam-gps-extract --output track.gpx file.mp4

    RUST_LOG=debug dashcam-gps-extract --force --sort gps --output test.gpx 'path/to/*F.mp4'
"#;

#[derive(Debug, StructOpt)]
#[structopt(name = "dashcam-gps-extract", about = ABOUT)]
#[structopt(setting = clap::AppSettings::ColoredHelp)]
pub struct Opts {
    /// Output file path
    #[structopt(short = "o", long, name = "output path", default_value = "dashcam.gpx")]
    pub output: PathBuf,

    /// Overwrite output file if exists
    #[structopt(short = "f", long)]
    pub force: bool,

    /// Sorting mode (file, gps, none)
    #[structopt(short = "s", long, default_value)]
    pub sort: SortingMode,

    /// Input file path or glob pattern
    #[structopt(name = "input path or glob pattern")]
    pub input: String,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum SortingMode {
    /// Sort the output based on the input file name(s)
    File,
    /// Sort the output based on the GPS date
    GpsDate,
    /// Don't sort the output
    None,
}

impl Default for SortingMode {
    fn default() -> Self {
        SortingMode::GpsDate
    }
}

impl fmt::Display for SortingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortingMode::File => f.write_str("file"),
            SortingMode::GpsDate => f.write_str("gps"),
            SortingMode::None => f.write_str("none"),
        }
    }
}

impl FromStr for SortingMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "file" => SortingMode::File,
            "gps" => SortingMode::GpsDate,
            "none" => SortingMode::None,
            _ => return Err("Unsupported sorting mode".to_string()),
        })
    }
}
