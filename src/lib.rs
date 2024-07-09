use std::{fmt::Display, io};

const PNG_HDR: &[u8] = &[137, 80, 78, 71, 13, 10, 26, 10];

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

    /// only method 0
    _compression_method: u8,

    /// only method 0
    _filter_method: u8,

    /// a boolean value to determine if image is interlaced or not
    /// the only available interlace type is "Adam7 interlace"
    interlace_method: bool,

    /// actual image data
    data: Vec<u8>,
}

impl Image {
    pub fn from(file: &str) -> io::Result<Self> {
        let mut helper = ImageHelper::from(file)?;
        let mut image = Self {
            width: 0,
            height: 0,
            bit_depth: 0,
            color_type: 0,
            _compression_method: 0,
            _filter_method: 0,
            interlace_method: false,
            data: Vec::new(),
        };

        loop {
            match Chunk::new(&mut helper)? {
                Chunk::IEND => {
                    break;
                }
                Chunk::IHDR(ihdr) => {
                    image.width = ihdr.width;
                    image.height = ihdr.height;
                    image.bit_depth = ihdr.bit_depth;
                    image.color_type = ihdr.color_type;
                    image._compression_method = ihdr._compression_method;
                    image._filter_method = ihdr._compression_method;
                    image.interlace_method = ihdr.interlace_method;
                }
                Chunk::IDAT(data) => {
                    // TODO: Decompress and de-filter
                    image.data = data.to_owned();
                }
                _ => {}
            }
        }

        Ok(image)
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: fix me
        write!(f, "Image dimention: {}x{}px", self.width, self.height)
    }
}

pub enum Chunk<'a> {
    IHDR(IHDRData),
    PLTE(PLTEData),
    IDAT(&'a [u8]),
    IEND,
    SBIT,
    TEXT,
}

impl<'a> Chunk<'a> {
    fn new(image: &'a mut ImageHelper) -> io::Result<Self> {
        let len_slice = &image.data[image.offset..image.offset + 4];
        let len = (len_slice[0] as u32) << (8 * 3)
            | (len_slice[1] as u32) << (8 * 2)
            | (len_slice[2] as u32) << (8 * 1)
            | (len_slice[3] as u32) << (8 * 0);
        image.offset += 4;

        // get type
        image.offset += 4;
        let chunk = match String::from_utf8_lossy(&image.data[image.offset - 4..image.offset])
            .to_string()
            .as_str()
        {
            "IHDR" => Self::IHDR(IHDRData::from(
                &image.data[image.offset..image.offset + len as usize],
            )),
            "PLTE" => Self::PLTE(PLTEData::from(
                &image.data[image.offset..image.offset + len as usize],
            )),
            "IDAT" => Self::IDAT(&image.data[image.offset..image.offset + len as usize]),
            "IEND" => Self::IEND,
            "sBIT" => Self::SBIT,
            "tEXt" => Self::TEXT,
            _ => {
                println!(
                    "Got {}",
                    String::from_utf8_lossy(&image.data[image.offset - 4..image.offset])
                );
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid PNG chunk",
                ));
            }
        };
        image.offset += len as usize;

        // get the CRC
        // TODO: Can actually ignore this
        let _crc = &image.data[image.offset..image.offset + 4];
        image.offset += 4;

        Ok(chunk)
    }

    pub fn print(&self) {
        match self {
            Chunk::IHDR(ihdr) => ihdr.print(),
            Chunk::PLTE(plte) => plte.print(),
            Chunk::IDAT(data) => println!("{:?}", *data),
            Chunk::IEND => println!("End of image chunk!"),
            Chunk::SBIT => println!("sBIT!"),
            Chunk::TEXT => println!("tEXt!"),
        }
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

    /// only method 0
    _compression_method: u8,

    /// only method 0
    _filter_method: u8,

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
            _compression_method: data[10],
            _filter_method: data[11],
            interlace_method: data[12] == 1,
        }
    }

    fn print(&self) {
        println!("Size: {}x{}px", self.width, self.height);
        println!("Bit Depth: {}", self.bit_depth);
        println!("Color Type: {}", self.color_type);
        println!(
            "Is interlaced? {}",
            if self.interlace_method { "Yes" } else { "No" }
        );
    }
}

pub struct PLTEData;
impl PLTEData {
    fn from(_data: &[u8]) -> Self {
        unimplemented!()
    }

    fn print(&self) {
        unimplemented!()
    }
}
