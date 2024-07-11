use std::{fmt::Display, io};

const PNG_HDR: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10];

macro_rules! pngerr {
    ($($args:tt)*) => {
        return Err(io::Error::new(io::ErrorKind::InvalidData, format!($($args)*)));
    };
}

struct ImageHelper {
    offset: usize,
    data: Vec<u8>,
}

impl ImageHelper {
    fn from(file: &str) -> io::Result<Self> {
        let data = std::fs::read(file)?;
        let pnghdr = &data[0..8];
        assert_eq!(pnghdr, PNG_HDR);

        Ok(Self { offset: 8, data })
    }

    fn next<'a>(&'a mut self) -> io::Result<Option<Chunk<'a>>> {
        let chunk = Chunk::new(self)?;
        if let Chunk::IEND = chunk {
            Ok(None)
        } else {
            Ok(Some(chunk))
        }
    }
}

#[derive(Debug)]
struct PLTEEntry {
    _red: u8,
    _green: u8,
    _blue: u8,
}

#[derive(Debug)]
enum Transparancy {
    Index(Vec<u8>),
    Grey(u16),
    RGB(u16, u16, u16),
}

impl Transparancy {
    fn for_indexed_color(data: &[u8], plte_len: usize) -> io::Result<Self> {
        if data.len() > plte_len {
            pngerr!("tRNS chunk has more entries than PLTE chunk");
        }

        let mut entries = data.to_owned();
        for _ in data.len()..plte_len {
            entries.push(255);
        }

        Ok(Self::Index(entries))
    }

    fn for_grayscale(data: &[u8]) -> io::Result<Self> {
        if data.len() != 2 {
            pngerr!("invalid tRNS chunk");
        }

        Ok(Self::Grey(u16::from_be_bytes([data[0], data[1]])))
    }

    fn for_rgb(data: &[u8]) -> io::Result<Self> {
        if data.len() != 6 {
            pngerr!("invalid tRNS chunk");
        }

        Ok(Self::RGB(
            u16::from_be_bytes([data[0], data[1]]),
            u16::from_be_bytes([data[2], data[3]]),
            u16::from_be_bytes([data[4], data[5]]),
        ))
    }
}

#[derive(Debug)]
pub struct Image {
    /// width in pixels
    width: u32,

    /// height in pixels
    height: u32,

    /// the number of bits per sample or per palette index (not per pixel)
    bit_depth: u8,

    /// the interpretation of the image data
    color_type: u8,

    /// a boolean value to determine if image is interlaced or not
    /// the only available interlace type is "Adam7 interlace"
    interlace_method: bool,

    /// actual image data
    data: Vec<u8>,

    /// palettes (PLTE chunk)
    plte: Option<Vec<PLTEEntry>>,

    /// background color
    background: Option<BKGD>,

    /// simple transparency
    transparancy: Option<Transparancy>,
}

impl Image {
    pub fn from(file: &str) -> io::Result<Self> {
        // let mut helper = ImageHelper::from(file)?;
        let mut chunks = ImageHelper::from(file)?;
        let mut image = Self {
            width: 0,
            height: 0,
            bit_depth: 0,
            color_type: 0,
            interlace_method: false,
            data: Vec::new(),
            plte: None,
            background: None,
            transparancy: None,
        };

        while let Some(chunk) = chunks.next()? {
            match chunk {
                Chunk::IEND => {
                    break;
                }
                Chunk::IHDR(ihdr) => {
                    image.width = ihdr.width;
                    image.height = ihdr.height;
                    image.bit_depth = ihdr.bit_depth;
                    image.color_type = ihdr.color_type;
                    image.interlace_method = ihdr.interlace_method;
                }
                Chunk::PLTE(plte) => {
                    // 4.1.2 - There must not be more than one PLTE chunk.
                    if image.plte.is_some() {
                        pngerr!("PNG must not have more than one PLTE chunk");
                    }

                    if image.background.is_some() {
                        pngerr!("bKGD chunk can not preceed a PLTE chunk");
                    }

                    image.plte = Some(plte);
                }
                Chunk::IDAT(data) => {
                    // TODO: Decompress and de-filter
                    image.data.extend(data);
                }
                Chunk::BKGD(background) => {
                    if !image.data.is_empty() {
                        pngerr!("bKGD chunk can not come after the IDAT chunk");
                    }

                    match image.color_type {
                        3 => {
                            if let BKGD::Index(_) = background {
                            } else {
                                pngerr!(
                                    "PNG with color type 3 can only have palette index bKGD chunk"
                                );
                            }
                        }
                        0 | 4 => {
                            if let BKGD::Grey(_) = background {
                            } else {
                                pngerr!("PNG with color type 0 or 4 can only have grey bKGD chunk");
                            }
                        }
                        2 | 6 => {
                            if let BKGD::RGB(_, _, _) = background {
                            } else {
                                pngerr!("PNG with color type 2 or 6 can only have RGB bKGD chunk");
                            }
                        }
                        _ => {}
                    }
                    image.background = Some(background);
                }
                Chunk::CHRM
                | Chunk::GAMA
                | Chunk::HIST
                | Chunk::PHYS
                | Chunk::SBIT
                | Chunk::TEXT
                | Chunk::TIME
                | Chunk::ZTXT => {
                    // ignore - not important in our use-case
                }
                Chunk::TRNS(data) => match image.color_type {
                    3 => {
                        image.transparancy = Some(Transparancy::for_indexed_color(
                            data,
                            image.plte.as_ref().expect("missing PLTE chunk").len(),
                        )?)
                    }
                    0 => image.transparancy = Some(Transparancy::for_grayscale(data)?),
                    2 => image.transparancy = Some(Transparancy::for_rgb(data)?),
                    4 | 6 => {
                        pngerr!("PNG with color types 4 or 6 can not have a tRNS chunk");
                    }
                    _ => {
                        unreachable!();
                    }
                },
            }
        }

        // TODO: validate the image
        // 4.1.2
        // This chunk must appear for color type 3, and can appear for
        // color types 2 and 6; it must not appear for color types 0 and
        // 4. If this chunk does appear, it must precede the first IDAT
        // chunk.
        match image.color_type {
            0 | 4 => {
                if image.plte.is_some() {
                    pngerr!("PNG with color type 0 or 4 cannot have a PLTE chunk");
                }
            }
            3 => {
                if image.plte.is_none() {
                    pngerr!("PNG with color type 3 must have a PLTE chunk");
                }
                let bit_depth_range = 2usize.pow(image.bit_depth as u32);
                if image.plte.as_ref().unwrap().len() > bit_depth_range {
                    pngerr!(
                        "PNG with color type 3 can not have more entries that its bit depth range"
                    );
                }
            }
            _ => {}
        }
        Ok(image)
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: fix me
        write!(
            f,
            "Dimention: {}x{}px\nColor Type: {}\nBit Depth: {}\nData Size: {} bytes",
            self.width,
            self.height,
            self.color_type,
            self.bit_depth,
            self.data.len()
        )
    }
}

#[derive(Debug)]
enum BKGD {
    Index(u8),
    Grey(u16),
    RGB(u16, u16, u16),
}

enum Chunk<'a> {
    IHDR(IHDRData),
    PLTE(Vec<PLTEEntry>),
    IDAT(&'a [u8]),
    IEND,
    BKGD(BKGD),
    CHRM,
    GAMA,
    HIST,
    PHYS,
    SBIT,
    TEXT,
    TIME,
    TRNS(&'a [u8]),
    ZTXT,
}

impl<'a> Chunk<'a> {
    fn new(image: &'a mut ImageHelper) -> io::Result<Self> {
        let len_slice = &image.data[image.offset..image.offset + 4];
        let len = ((len_slice[0] as u32) << (8 * 3)
            | (len_slice[1] as u32) << (8 * 2)
            | (len_slice[2] as u32) << (8 * 1)
            | (len_slice[3] as u32) << (8 * 0)) as usize;
        image.offset += 4;

        // get type
        image.offset += 4;
        let data = &image.data[image.offset..image.offset + len];
        let chunk = match String::from_utf8_lossy(&image.data[image.offset - 4..image.offset])
            .to_string()
            .as_str()
        {
            "IHDR" => Self::IHDR(IHDRData::from(data)),
            "PLTE" => {
                if len % 3 != 0 {
                    pngerr!("invalid PLTE chunk");
                }

                let mut entries = Vec::new();
                let mut idx = 0usize;
                loop {
                    if idx == len as usize {
                        break;
                    }

                    entries.push(PLTEEntry {
                        _red: data[idx],
                        _green: data[idx + 1],
                        _blue: data[idx + 2],
                    });

                    idx += 3;
                }
                Self::PLTE(entries)
            }
            "IDAT" => Self::IDAT(data),
            "IEND" => Self::IEND,
            "bKGD" => match len {
                1 => Self::BKGD(BKGD::Index(data[0])),
                2 => Self::BKGD(BKGD::Grey(u16::from_be_bytes([data[0], data[1]]))),
                6 => Self::BKGD(BKGD::RGB(
                    u16::from_be_bytes([data[0], data[1]]),
                    u16::from_be_bytes([data[2], data[3]]),
                    u16::from_be_bytes([data[4], data[5]]),
                )),
                _ => {
                    pngerr!("invalid bKGD chunk");
                }
            },
            "cHRM" => Self::CHRM,
            "gAMA" => Self::GAMA,
            "hIST" => Self::HIST,
            "pHYs" => Self::PHYS,
            "sBIT" => Self::SBIT,
            "tEXt" => Self::TEXT,
            "tIME" => Self::TIME,
            "tRNS" => Self::TRNS(data),
            "zTXT" => Self::ZTXT,
            _ => {
                pngerr!("{} is an invalid PNG chunk", String::from_utf8_lossy(data));
            }
        };
        image.offset += len as usize;

        // get the CRC
        // TODO: Can actually ignore this
        let _crc = &image.data[image.offset..image.offset + 4];
        image.offset += 4;

        Ok(chunk)
    }
}

/// IHDR Chunk - RFC 2083 (section 4.1.1)
pub struct IHDRData {
    /// width in pixels
    width: u32,

    /// height in pixels
    height: u32,

    /// the number of bits per sample or per palette index (not per pixel)
    bit_depth: u8,

    /// the interpretation of the image data
    color_type: u8,

    /// a boolean value to determine if image is interlaced or not
    /// the only available interlace type is "Adam7 interlace"
    interlace_method: bool,
}

impl IHDRData {
    fn from(data: &[u8]) -> Self {
        Self {
            width: (data[0] as u32) << (8 * 3)
                | (data[1] as u32) << (8 * 2)
                | (data[2] as u32) << (8 * 1)
                | (data[3] as u32) << (8 * 0),
            height: (data[4] as u32) << (8 * 3)
                | (data[5] as u32) << (8 * 2)
                | (data[6] as u32) << (8 * 1)
                | (data[7] as u32) << (8 * 0),
            bit_depth: data[8],
            color_type: data[9],
            // ignore filter and compression methods since there is only one
            interlace_method: data[12] == 1,
        }
    }
}
