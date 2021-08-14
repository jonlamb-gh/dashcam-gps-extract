use byteorder::{BigEndian, ByteOrder, LittleEndian};
use std::str;

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum LongitudeHemisphere {
    East,
    West,
}

#[derive(Debug, Clone)]
pub struct NovatekGps<T: AsRef<[u8]>> {
    buffer: T,
}

mod field {
    type Field = ::core::ops::Range<usize>;

    pub const BOX_SIZE: Field = 0..4;
    pub const BOX_TYPE: Field = 4..8;
    pub const MAGIC: Field = 8..12;
    pub const HR: Field = 16..20;
    pub const MIN: Field = 20..24;
    pub const SEC: Field = 24..28;
    pub const SAT_LOCK: usize = 40;
    pub const LAT_HEMI: usize = 41;
    pub const LON_HEMI: usize = 42;
}

impl<T: AsRef<[u8]>> NovatekGps<T> {
    pub const MIN_SIZE: usize = 128; // TODO - not sure yet, use field once done
    pub const BOX_TYPE: &'static str = "free";
    pub const MAGIC_WORD: &'static str = "GPS ";

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
}
