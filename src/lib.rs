use flate2::bufread::ZlibDecoder;
use std::{
    fmt::Display,
    io::{self, Read},
};

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
    PaletteIndex(Vec<u8>),
    Greyscale(u16),
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

        Ok(Self::PaletteIndex(entries))
    }

    fn for_grayscale(data: &[u8]) -> io::Result<Self> {
        if data.len() != 2 {
            pngerr!("invalid tRNS chunk");
        }

        Ok(Self::Greyscale(u16::from_be_bytes([data[0], data[1]])))
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
enum ColorType {
    Greyscale,
    RGB,
    PaletteIndex,
    GreyscaleAlpha,
    RGBA,
}

impl Display for ColorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color_type = match self {
            Self::Greyscale => "Greyscale",
            Self::RGB => "RGB",
            Self::PaletteIndex => "Palette Index",
            Self::GreyscaleAlpha => "Greyscale with Alpha",
            Self::RGBA => "RGB with Alpha",
        };
        write!(f, "{}", color_type)
    }
}

pub struct Img {
    grid: Vec<Vec<u8>>,
}

impl Img {
    pub fn new(file: &str) -> io::Result<Self> {
        let image = Image::from(file)?;
        let mut grid = Vec::new();
        let pixle_size = match image.color_type {
            ColorType::Greyscale => {
                if image.bit_depth == 16 {
                    unimplemented!("only 8bit colors are supported");
                } else {
                    1
                }
            }
            ColorType::RGB => {
                if image.bit_depth == 8 {
                    3
                } else {
                    unimplemented!("only 8bit colors are supported");
                }
            }
            ColorType::PaletteIndex => 1,
            ColorType::GreyscaleAlpha => {
                if image.bit_depth == 8 {
                    2
                } else {
                    unimplemented!("only 8bit colors are supported");
                }
            }
            ColorType::RGBA => {
                if image.bit_depth == 8 {
                    4
                } else {
                    unimplemented!("only 8bit colors are supported");
                }
            }
        };

        for r in 0..image.height {
            let mut row = Vec::new();
            for c in 0..image.width {
                let idx = (c * pixle_size) as usize + (r * image.width) as usize;
                let value = match image.color_type {
                    ColorType::Greyscale => image.data[idx],
                    ColorType::RGB => {
                        ((image.data[idx] as u32
                            + image.data[idx + 1] as u32
                            + image.data[idx + 2] as u32)
                            / 3) as u8
                    }
                    ColorType::PaletteIndex => {
                        let plte = image.plte.as_ref().unwrap();
                        let entry = &plte[image.data[idx] as usize];

                        ((entry._red as u32 + entry._green as u32 + entry._blue as u32) / 3) as u8
                    }
                    ColorType::GreyscaleAlpha => {
                        ((image.data[idx] as u16 + image.data[idx + 1] as u16) / 2) as u8
                    }
                    ColorType::RGBA => {
                        ((image.data[idx] as u32
                            + image.data[idx + 1] as u32
                            + image.data[idx + 2] as u32
                            + image.data[idx + 3] as u32)
                            / 4) as u8
                    }
                };
                row.push(value)
            }
            grid.push(row);
        }

        Ok(Self { grid })
    }

    pub fn display(&self) {
        // TODO: get the average of 15x30 pixles into a single pixle in the case of 1920x1080
        let mut resized: Vec<Vec<u8>> = Vec::new();
        let fact = 6;
        for r in 0..self.grid.len() / (fact * 2) {
            let mut row = Vec::new();
            for c in 0..self.grid[0].len() / fact {
                let start_r = r * fact * 2;
                let start_c = c * fact;
                let total = 11f32 * 22f32;
                let mut ave = 0f32;
                for i in 0..fact * 2 {
                    let idx_y = start_r + i;
                    for j in 0..fact {
                        let idx_x = start_c + j;
                        ave += self.grid[idx_y][idx_x] as f32 / total;
                    }
                }
                row.push(ave as u8);
            }
            resized.push(row);
        }

        let chars: Vec<char> =
            " `^\",:;Il!i~+_-?][}{1)(|\\/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"
                .chars()
                .collect();
        for row in resized {
            for darkness in row {
                let idx = ((66u16 * darkness as u16) / 255) as usize;
                let idx = if idx > 65 { 65 } else { idx };
                print!("{}", chars[idx]);
            }
            println!();
        }
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
    color_type: ColorType,

    /// a boolean value to determine if image is interlaced or not
    /// the only available interlace type is "Adam7 interlace"
    interlaced: bool,

    /// actual image data
    data: Vec<u8>,

    /// palettes (PLTE chunk)
    plte: Option<Vec<PLTEEntry>>,

    /// background color (bKGD chunk)
    background: Option<BKGD>,

    /// simple transparency (tRNS chunk)
    transparancy: Option<Transparancy>,
}

impl Image {
    pub fn from(file: &str) -> io::Result<Self> {
        let mut chunks = ImageHelper::from(file)?;
        let mut image = Self {
            width: 0,
            height: 0,
            bit_depth: 0,
            color_type: ColorType::Greyscale,
            interlaced: false,
            data: Vec::new(),
            plte: None,
            background: None,
            transparancy: None,
        };
        let mut compressed_data: Vec<u8> = Vec::new();

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
                    image.interlaced = ihdr.interlace_method;
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
                    compressed_data.extend(data);
                }
                Chunk::BKGD(background) => {
                    if !compressed_data.is_empty() {
                        pngerr!("bKGD chunk can not come after the IDAT chunk");
                    }

                    match image.color_type {
                        ColorType::PaletteIndex => {
                            if let BKGD::PaletteIndex(_) = background {
                            } else {
                                pngerr!(
                                    "PNG with color type 3 can only have palette index bKGD chunk"
                                );
                            }
                        }
                        ColorType::Greyscale | ColorType::GreyscaleAlpha => {
                            if let BKGD::Greyscale(_) = background {
                            } else {
                                pngerr!("PNG with color type 0 or 4 can only have grey bKGD chunk");
                            }
                        }
                        ColorType::RGB | ColorType::RGBA => {
                            if let BKGD::RGB(_, _, _) = background {
                            } else {
                                pngerr!("PNG with color type 2 or 6 can only have RGB bKGD chunk");
                            }
                        }
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
                    ColorType::PaletteIndex => {
                        image.transparancy = Some(Transparancy::for_indexed_color(
                            data,
                            image.plte.as_ref().expect("missing PLTE chunk").len(),
                        )?)
                    }
                    ColorType::Greyscale => {
                        image.transparancy = Some(Transparancy::for_grayscale(data)?)
                    }
                    ColorType::RGB => image.transparancy = Some(Transparancy::for_rgb(data)?),
                    ColorType::GreyscaleAlpha | ColorType::RGBA => {
                        pngerr!("PNG with color types 4 or 6 can not have a tRNS chunk");
                    }
                },
            }
        }

        // 4.1.2
        // This chunk must appear for color type 3, and can appear for
        // color types 2 and 6; it must not appear for color types 0 and
        // 4. If this chunk does appear, it must precede the first IDAT
        // chunk.
        match image.color_type {
            ColorType::Greyscale => {
                // validate bit depth
                if let 1 | 2 | 4 | 8 | 16 = image.bit_depth {
                } else {
                    pngerr!(
                        "PNG of {} color type must have bit depth of 1, 2, 4, 8, or 16",
                        image.color_type
                    );
                }

                // validate PLTE chunk existance
                if image.plte.is_some() {
                    pngerr!(
                        "PNG of {} color type cannot have a PLTE chunk",
                        image.color_type
                    );
                }
            }
            ColorType::GreyscaleAlpha => {
                // validate bit depth
                if let 8 | 16 = image.bit_depth {
                } else {
                    pngerr!(
                        "PNG of {} color type must have bit depth of 8 or 16",
                        image.color_type
                    );
                }

                // validate PLTE chunk existance
                if image.plte.is_some() {
                    pngerr!(
                        "PNG of {} color type cannot have a PLTE chunk",
                        image.color_type
                    );
                }
            }
            ColorType::PaletteIndex => {
                // validate bit depth
                if let 1 | 2 | 4 | 8 = image.bit_depth {
                } else {
                    pngerr!(
                        "PNG of {} color type must have bit depth of 1, 2, 4, or 8",
                        image.color_type
                    );
                }

                // validate PLTE chunk existance
                if image.plte.is_none() {
                    pngerr!(
                        "PNG of {} color type must have a PLTE chunk",
                        image.color_type
                    );
                }

                // validate palette entry length
                let bit_depth_range = 2usize.pow(image.bit_depth as u32);
                if image.plte.as_ref().unwrap().len() > bit_depth_range {
                    pngerr!(
                        "PNG of {} color type can not have more entries that its bit depth range",
                        image.color_type
                    );
                }
            }
            ColorType::RGB | ColorType::RGBA => {
                // validate bit depth
                if let 8 | 16 = image.bit_depth {
                } else {
                    pngerr!(
                        "PNG of {} color type must have bit depth of 1, 2, 4, or 8",
                        image.color_type
                    );
                }
            }
        }

        // decompress data
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut filtered = Vec::new();
        decoder.read_to_end(&mut filtered)?;

        reverse_filter(filtered, &mut image)?;

        Ok(image)
    }
}

#[derive(Debug)]
enum FilterType {
    None,
    Sub,
    Up,
    Average,
    Paeth,
}

impl FilterType {
    fn from(byte: u8) -> io::Result<Self> {
        match byte {
            0 => Ok(Self::None),
            1 => Ok(Self::Sub),
            2 => Ok(Self::Up),
            3 => Ok(Self::Average),
            4 => Ok(Self::Paeth),
            _ => {
                pngerr!("invalid filter type");
            }
        }
    }
}

/// RFC 2083 - Section 6
fn reverse_filter(filtered: Vec<u8>, image: &mut Image) -> io::Result<()> {
    let width = filtered.len() / image.height as usize;
    let bpp = match image.color_type {
        ColorType::Greyscale => {
            if image.bit_depth == 16 {
                2
            } else {
                1
            }
        }
        ColorType::RGB => {
            if image.bit_depth == 8 {
                3
            } else {
                6
            }
        }
        ColorType::PaletteIndex => 1,
        ColorType::GreyscaleAlpha => {
            if image.bit_depth == 8 {
                2
            } else {
                4
            }
        }
        ColorType::RGBA => {
            if image.bit_depth == 8 {
                4
            } else {
                8
            }
        }
    };

    for r in 0..image.height as usize {
        match FilterType::from(filtered[r * width])? {
            FilterType::None => {
                image
                    .data
                    .extend(&filtered[((r * width) + 1)..((r * width) + width)]);
            }
            FilterType::Sub => {
                // CHECK: Section 6.3: Raw(x) = Sub(x) + Raw(x-bpp)
                for c in 1..width {
                    let x = (r * width) + c;
                    if c < bpp + 1 {
                        image.data.push(filtered[x]);
                        continue;
                    }

                    image
                        .data
                        .push(filtered[x].wrapping_add(image.data[x - bpp - 1 - r]));
                }
            }
            FilterType::Up => {
                // CHECK: Section 6.4: Raw(x) = Up(x) + Prior(x)
                for c in 1..width {
                    let x = (r * width) + c;
                    let prior = (r * (width - 1)) + c;
                    if r == 0 {
                        image.data.push(filtered[x]);
                        continue;
                    }

                    image
                        .data
                        .push(filtered[x].wrapping_add(filtered[x - prior]));
                }
            }
            FilterType::Average => {
                // CHECK: Section 6.5: Raw(x) = Average(x) + floor((Raw(x-bpp)+Prior(x))/2)
                for c in 1..width {
                    let x = (r * width) + c;
                    let raw_x_bpp = if c < bpp + 1 {
                        0
                    } else {
                        filtered[x - bpp - 1 - r]
                    };
                    let prior = if r == 0 {
                        0
                    } else {
                        filtered[(r * (width - 1)) + c]
                    };
                    let floor = ((raw_x_bpp as u16 + prior as u16) / 2) as u8;
                    image.data.push(filtered[x].wrapping_add(floor));
                }
            }
            FilterType::Paeth => {
                // CHECK: Section 6.6: Raw(x) = Paeth(x) + PaethPredictor(Raw(x-bpp), Prior(x), Prior(x-bpp))
                for c in 1..width {
                    let x = (r * width) + c;
                    let top_idx = (r * (width - 1)) + c;

                    let left = if bpp > x {
                        0
                    } else {
                        filtered[x - bpp - 1 - r]
                    };
                    let top = if r == 0 { 0 } else { filtered[top_idx] };
                    let top_left = if r == 0 || c < bpp + 1 {
                        0
                    } else {
                        filtered[top_idx - bpp]
                    };
                    let predictor = paeth_predictor(left, top, top_left);
                    image.data.push(filtered[x].wrapping_add(predictor));
                }
            }
        }
    }

    Ok(())
}

fn paeth_predictor(left: u8, top: u8, top_left: u8) -> u8 {
    let p = left as i16 + top as i16 - top_left as i16;
    let pleft = 0i16.abs_diff(p - left as i16);
    let ptop = 0i16.abs_diff(p - top as i16);
    let ptop_left = 0i16.abs_diff(p - top_left as i16);

    if pleft <= ptop && pleft <= ptop_left {
        left
    } else if ptop <= ptop_left {
        top
    } else {
        top_left
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Dimention: {}x{}px\nColor Type: {}\nBit Depth: {}\nInterlaced: {}\nData Size: {} bytes",
            self.width,
            self.height,
            self.color_type,
            self.bit_depth,
            if self.interlaced { "Yes" } else { "No" },
            self.data.len()
        )
    }
}

#[derive(Debug)]
enum BKGD {
    PaletteIndex(u8),
    Greyscale(u16),
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
            "IHDR" => Self::IHDR(IHDRData::from(data)?),
            "PLTE" => {
                if len % 3 != 0 {
                    pngerr!("invalid PLTE chunk");
                }

                let mut entries = Vec::new();
                let mut idx = 0usize;
                loop {
                    if idx == len {
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
            "IEND" => {
                if len != 0 {
                    pngerr!("IEND chunk must not contain any data");
                }
                Self::IEND
            }
            "bKGD" => match len {
                1 => Self::BKGD(BKGD::PaletteIndex(data[0])),
                2 => Self::BKGD(BKGD::Greyscale(u16::from_be_bytes([data[0], data[1]]))),
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
        image.offset += len;

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
    color_type: ColorType,

    /// a boolean value to determine if image is interlaced or not
    /// the only available interlace type is "Adam7 interlace"
    interlace_method: bool,
}

impl IHDRData {
    fn from(data: &[u8]) -> io::Result<Self> {
        let idhr = Self {
            width: (data[0] as u32) << (8 * 3)
                | (data[1] as u32) << (8 * 2)
                | (data[2] as u32) << (8 * 1)
                | (data[3] as u32) << (8 * 0),
            height: (data[4] as u32) << (8 * 3)
                | (data[5] as u32) << (8 * 2)
                | (data[6] as u32) << (8 * 1)
                | (data[7] as u32) << (8 * 0),
            bit_depth: data[8],
            color_type: match data[9] {
                0 => ColorType::Greyscale,
                2 => ColorType::RGB,
                3 => ColorType::PaletteIndex,
                4 => ColorType::GreyscaleAlpha,
                6 => ColorType::RGBA,
                _ => {
                    pngerr!("invalid color type");
                }
            },
            // ignore filter and compression methods since there is only one
            interlace_method: data[12] == 1,
        };

        Ok(idhr)
    }
}
