// TODO
// #![deny(warnings, clippy::all)]

use crate::error::Error;
use crate::novatek_gps::NovatekGps;
use crate::opts::Opts;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use mp4::{GpsBox, Mp4Box, Mp4Reader};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::process;
use structopt::StructOpt;

mod error;
mod novatek_gps;
mod opts;

fn main() {
    match do_main() {
        Ok(()) => (),
        Err(e) => {
            log::error!("{}", e);
            process::exit(exitcode::SOFTWARE);
        }
    }
}

fn do_main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();
    let opts = Opts::from_args();

    if !opts.force && opts.output.exists() {
        return Err(Error::OutputFileExists(opts.output));
    }

    let output_file = File::create(&opts.output)?;

    let input_file = File::open(&opts.input)?;
    let input_size = input_file.metadata()?.len();
    let input_reader = BufReader::new(input_file);
    let mp4 = Mp4Reader::read_header(input_reader, input_size)?;

    let gps_box = if let Some(gps) = &mp4.moov.gps {
        gps.clone()
    } else {
        log::warn!("No GPS blocks in {:?}", opts.input);
        return Ok(());
    };

    let file_data = FileData {
        file_name: opts
            .input
            .file_name()
            .ok_or_else(|| Error::PathNotFile(opts.input.clone()))?
            .to_string_lossy()
            .to_string(),
        gps: gps_box,
    };
    log::info!(
        "Loaded '{}', {}",
        file_data.file_name,
        file_data.gps.summary()?
    );

    let mut reader = mp4.into_inner();

    let mut buf = Vec::with_capacity(0x1000);

    for (idx, b) in file_data.gps.data_blocks.iter().enumerate() {
        log::debug!("[{}] 0x{:08X}, size={}", idx, b.offset, b.size,);

        reader.seek(SeekFrom::Start(b.offset.into()))?;

        buf.clear();
        buf.resize(b.size as usize, 0);
        reader.read_exact(&mut buf)?;

        let gps = match NovatekGps::new(&buf[..]) {
            Ok(gps) => gps,
            Err(e) => {
                log::warn!(
                    "Skipping GPS block [{}] at offset 0x{:08X} size={}: {}",
                    idx,
                    b.offset,
                    b.size,
                    e,
                );
                continue;
            }
        };

        println!(
            "{}-{}-{} {}:{}:{} == {}",
            gps.year(),
            gps.month(),
            gps.day(),
            gps.hour(),
            gps.minute(),
            gps.second(),
            gps.datetime(),
        );
        let lat_hemi = gps.latitude_hemisphere()?;
        let lat_dms = gps.latitude();
        let lat = gps.latitude_deg()?;
        println!("Lat {}, DMS {}, deg {}", lat_hemi, lat_dms, lat);

        let lon_hemi = gps.longitude_hemisphere()?;
        let lon_dms = gps.longitude();
        let lon = gps.longitude_deg()?;
        println!("Lon {}, DMS {}, deg {}", lon_hemi, lon_dms, lon);

        println!("Speed {}, bearing {}", gps.speed_mps(), gps.bearing());
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileData {
    pub file_name: String,
    pub gps: GpsBox,
}
