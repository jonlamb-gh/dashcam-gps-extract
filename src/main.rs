// TODO
// #![deny(warnings, clippy::all)]

use crate::error::Error;
use crate::novatek_gps::NovatekGps;
use crate::opts::Opts;
use gpx::*;
use mp4::{Mp4Box, Mp4Reader};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::{fmt, process};
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

    let mut gpx = Gpx::default();
    gpx.version = GpxVersion::Gpx11;
    gpx.creator = env!("CARGO_PKG_NAME").to_string().into();
    gpx.metadata = Metadata {
        name: opts
            .output
            .file_name()
            .ok_or_else(|| Error::PathNotFile(opts.output.clone()))?
            .to_string_lossy()
            .to_string()
            .into(),
        time: chrono::Utc::now().into(),
        ..Default::default()
    }
    .into();

    let output_file = File::create(&opts.output)?;

    // TODO - gather input files, order/sort, for-each
    // sort by filename happens here, sort by gps time happens in the vec after collection
    let input_files = vec![&opts.input];
    let mut gps_items = Vec::new();

    let mut buf = Vec::with_capacity(0x1000);

    for input in input_files.into_iter() {
        let file_name = input
            .file_name()
            .ok_or_else(|| Error::PathNotFile(input.clone()))?
            .to_string_lossy()
            .to_string();
        let input_file = File::open(input)?;
        let input_size = input_file.metadata()?.len();
        let input_reader = BufReader::new(input_file);
        let mp4 = Mp4Reader::read_header(input_reader, input_size)?;

        let gps_box = if let Some(gps) = &mp4.moov.gps {
            gps.clone()
        } else {
            log::warn!("No GPS blocks in {:?}", input);
            continue;
        };

        log::info!("Loaded '{}', {}", file_name, gps_box.summary()?);

        let mut reader = mp4.into_inner();

        for (idx, b) in gps_box.data_blocks.iter().enumerate() {
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

            let gps_data = GpsData {
                file_name: file_name.clone(),
                datetime: gps.datetime(),
                latitude: gps.latitude_deg()?,
                longitude: gps.longitude_deg()?,
                speed: gps.speed_mps(),
                course: gps.bearing(),
            };
            log::info!("{}", gps_data);
            gps_items.push(gps_data);
        }
    }

    gps_items.sort_by_key(|g| g.datetime);

    // TODO - segment the TrackSegments when GPS data sat lock is not valid
    // currently single Track with all items in a single TrackSegment
    // fill out all the Waypoint fields
    let points = gps_items
        .into_iter()
        .map(|gps| {
            let mut wp = Waypoint::new((gps.longitude, gps.latitude).into());
            // TODO timezone in opts
            wp.time = chrono::DateTime::from_utc(gps.datetime, chrono::Utc).into();
            wp.source = gps.file_name.into();
            wp.speed = gps.speed.into();
            wp.fix = Fix::TwoDimensional.into();
            wp.sat = 3.into();
            wp
        })
        .collect();

    let segment = TrackSegment { points };

    gpx.tracks = vec![Track {
        segments: vec![segment],
        ..Default::default()
    }];

    gpx::write(&gpx, output_file)?;

    Ok(())
}

#[derive(Debug, Clone)]
struct GpsData {
    file_name: String,
    datetime: chrono::NaiveDateTime,
    latitude: f64,
    longitude: f64,
    speed: f64,
    course: f32,
}

impl fmt::Display for GpsData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}, {}, {} m/s, {}°",
            self.datetime, self.latitude, self.longitude, self.speed, self.course
        )
    }
}
