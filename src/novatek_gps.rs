use byteorder::{BigEndian, ByteOrder, LittleEndian};
use chrono::{NaiveDate, NaiveDateTime};
use std::{fmt, str};

#[derive(Debug, Clone, Eq, PartialEq, err_derive::Error)]
pub enum Error {
    #[error(display = "Buffer too small")]
    MissingBytes,

    #[error(display = "Invalid buffer length {} for box size {}", _0, _1)]
    InvalidBoxSize(usize, usize),

    #[error(display = "UTF8 error")]
    Uft8(#[error(source)] str::Utf8Error),

    #[error(display = "Invalid box type '{}', expected '{}'", _0, 1)]
    InvalidBoxType(String, &'static str),

    #[error(display = "Invalid magic word '{}', expected '{}'", _0, 1)]
    InvalidMagicWord(String, &'static str),

    #[error(display = "No satelite lock")]
    NoSatLock,

    #[error(display = "Invalid latitude (N/S) or longitude (E/W) hemisphere")]
    InvalidHemisphere,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LatitudeHemisphere {
    North,
    South,
}

impl fmt::Display for LatitudeHemisphere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LongitudeHemisphere {
    East,
    West,
}

impl fmt::Display for LongitudeHemisphere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct NovatekGps<T: AsRef<[u8]>> {
    buffer: T,
}

mod field {
    type Field = ::core::ops::Range<usize>;
    type Rest = ::core::ops::RangeFrom<usize>;

    pub const BOX_SIZE: Field = 0..4;
    pub const BOX_TYPE: Field = 4..8;
    pub const MAGIC: Field = 8..12;
    pub const HR: Field = 16..20;
    pub const MIN: Field = 20..24;
    pub const SEC: Field = 24..28;
    pub const YEAR: Field = 28..32;
    pub const MONTH: Field = 32..36;
    pub const DAY: Field = 36..40;
    pub const SAT_LOCK: usize = 40;
    pub const LAT_HEMI: usize = 41;
    pub const LON_HEMI: usize = 42;
    pub const LAT: Field = 44..48;
    pub const LON: Field = 48..52;
    pub const SPEED: Field = 52..56;
    pub const BEARING: Field = 56..60;
    pub const REST: Rest = 60..;
}

impl<T: AsRef<[u8]>> NovatekGps<T> {
    pub const MIN_SIZE: usize = field::REST.start;
    pub const BOX_TYPE: &'static str = "free";
    pub const MAGIC_WORD: &'static str = "GPS ";
    pub const YEAR_OFFSET: u32 = 2000;

    pub fn new_unchecked(buffer: T) -> NovatekGps<T> {
        NovatekGps { buffer }
    }

    pub fn new(buffer: T) -> Result<NovatekGps<T>, Error> {
        let g = Self::new_unchecked(buffer);
        g.check_len()?;
        g.check_box_size()?;
        g.check_box_type()?;
        g.check_magic_word()?;
        g.check_sat_lock()?;
        g.check_hemisphere()?;
        Ok(g)
    }

    pub fn check_len(&self) -> Result<(), Error> {
        let len = self.buffer.as_ref().len();
        if len < Self::MIN_SIZE {
            Err(Error::MissingBytes)
        } else {
            Ok(())
        }
    }

    pub fn check_box_size(&self) -> Result<(), Error> {
        let box_size = self.box_size() as usize;
        let buf_len = self.buffer.as_ref().len();
        if box_size != buf_len {
            Err(Error::InvalidBoxSize(buf_len, box_size))
        } else {
            Ok(())
        }
    }

    pub fn check_box_type(&self) -> Result<(), Error> {
        let box_type = self.box_type()?;
        if box_type != Self::BOX_TYPE {
            Err(Error::InvalidBoxType(box_type.to_string(), Self::BOX_TYPE))
        } else {
            Ok(())
        }
    }

    pub fn check_magic_word(&self) -> Result<(), Error> {
        let magic = self.magic_word()?;
        if magic != Self::MAGIC_WORD {
            Err(Error::InvalidMagicWord(magic.to_string(), Self::MAGIC_WORD))
        } else {
            Ok(())
        }
    }

    pub fn check_sat_lock(&self) -> Result<(), Error> {
        if self.sat_lock() {
            Ok(())
        } else {
            Err(Error::NoSatLock)
        }
    }

    pub fn check_hemisphere(&self) -> Result<(), Error> {
        let _ = self.latitude_hemisphere()?;
        let _ = self.longitude_hemisphere()?;
        Ok(())
    }

    #[inline]
    pub fn box_size(&self) -> u32 {
        let data = self.buffer.as_ref();
        BigEndian::read_u32(&data[field::BOX_SIZE])
    }

    #[inline]
    pub fn box_type(&self) -> Result<&str, Error> {
        let data = self.buffer.as_ref();
        let typ = str::from_utf8(&data[field::BOX_TYPE])?;
        Ok(typ)
    }

    #[inline]
    pub fn magic_word(&self) -> Result<&str, Error> {
        let data = self.buffer.as_ref();
        let m = str::from_utf8(&data[field::MAGIC])?;
        Ok(m)
    }

    #[inline]
    pub fn hour(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::HR])
    }

    #[inline]
    pub fn minute(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::MIN])
    }

    #[inline]
    pub fn second(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::SEC])
    }

    #[inline]
    pub fn year(&self) -> u32 {
        let data = self.buffer.as_ref();
        Self::YEAR_OFFSET + LittleEndian::read_u32(&data[field::YEAR])
    }

    #[inline]
    pub fn month(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::MONTH])
    }

    #[inline]
    pub fn day(&self) -> u32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_u32(&data[field::DAY])
    }

    #[inline]
    pub fn datetime(&self) -> NaiveDateTime {
        NaiveDate::from_ymd(self.year() as _, self.month(), self.day()).and_hms(
            self.hour(),
            self.minute(),
            self.second(),
        )
    }

    #[inline]
    pub fn sat_lock(&self) -> bool {
        let data = self.buffer.as_ref();
        let b = data[field::SAT_LOCK];
        // 0x41 == 'A', locked
        b == 0x41
    }

    #[inline]
    pub fn latitude_hemisphere(&self) -> Result<LatitudeHemisphere, Error> {
        let data = self.buffer.as_ref();
        let h = data[field::LAT_HEMI];
        match h {
            // 'N'
            0x4E => Ok(LatitudeHemisphere::North),
            // 'S'
            0x53 => Ok(LatitudeHemisphere::South),
            _ => Err(Error::InvalidHemisphere),
        }
    }

    #[inline]
    pub fn longitude_hemisphere(&self) -> Result<LongitudeHemisphere, Error> {
        let data = self.buffer.as_ref();
        let h = data[field::LON_HEMI];
        match h {
            // 'E'
            0x45 => Ok(LongitudeHemisphere::East),
            // 'W'
            0x57 => Ok(LongitudeHemisphere::West),
            _ => Err(Error::InvalidHemisphere),
        }
    }

    /// DDDmm.mmmm D=degrees m=minutes
    #[inline]
    pub fn latitude(&self) -> f32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_f32(&data[field::LAT])
    }

    #[inline]
    pub fn latitude_deg(&self) -> Result<f64, Error> {
        let hemi = self.latitude_hemisphere()?;
        let invert = matches!(hemi, LatitudeHemisphere::South);
        Ok(Self::dms_to_deg(self.latitude() as f64, invert))
    }

    /// DDDmm.mmmm D=degrees m=minutes
    #[inline]
    pub fn longitude(&self) -> f32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_f32(&data[field::LON])
    }

    #[inline]
    pub fn longitude_deg(&self) -> Result<f64, Error> {
        let hemi = self.longitude_hemisphere()?;
        let invert = matches!(hemi, LongitudeHemisphere::West);
        Ok(Self::dms_to_deg(self.longitude() as f64, invert))
    }

    /// Knots
    #[inline]
    pub fn speed(&self) -> f32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_f32(&data[field::SPEED])
    }

    /// m/s
    #[inline]
    pub fn speed_mps(&self) -> f64 {
        self.speed() as f64 * 0.514444
    }

    /// Degrees
    #[inline]
    pub fn bearing(&self) -> f32 {
        let data = self.buffer.as_ref();
        LittleEndian::read_f32(&data[field::BEARING])
    }

    #[inline]
    fn dms_to_deg(dms: f64, invert: bool) -> f64 {
        let min = dms % 100.0;
        let deg = dms - min;
        let out = deg / 100.0 + (min / 60.0);
        if invert {
            -1.0 * out
        } else {
            out
        }
    }
}
