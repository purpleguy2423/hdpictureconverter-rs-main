use std::io::{BufRead, Write};
use std::io::{Result as IoResult, Seek};
use std::iter::repeat;

use image::{Rgba, RgbaImage};
use imagequant::RGBA;
use rgb::FromSlice;
use tifiles::VariableType;

pub struct Image {
    input: RgbaImage,
    var_prefix: String,
    name: String,
}

/// Compute the integer ceiling of `lhs / rhs`
fn div_ceil(lhs: u32, rhs: u32) -> u32 {
    (lhs + rhs - 1) / rhs
}

impl Image {
    const TILE_SIZE: u32 = 80;

    pub fn new<R: BufRead + Seek>(data: R, name: &str, var_prefix: &str) -> IoResult<Self> {
        let loaded_image = match image::io::Reader::new(data).with_guessed_format()?.decode() {
            Ok(i) => i,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unable to decode image: {}", e),
                ));
            }
        };

        // Generate a black image that's rounded to a multiple of TILE_SIZE
        let mut image = image::ImageBuffer::from_pixel(
            div_ceil(loaded_image.width(), Self::TILE_SIZE) * Self::TILE_SIZE,
            div_ceil(loaded_image.height(), Self::TILE_SIZE) * Self::TILE_SIZE,
            Rgba([0u8, 0, 0, 255]),
        );

        // Paste the loaded image onto the black canvas, which also blends down so we know
        // we're fully opaque.
        image::imageops::overlay(&mut image, &loaded_image, 0, 0);
        assert_eq!(
            image.height() % Self::TILE_SIZE,
            0,
            "Image height {} isn't a multiple of tile size",
            image.height()
        );
        assert_eq!(
            image.width() % Self::TILE_SIZE,
            0,
            "Image width {} isn't a multiple of tile size",
            image.width()
        );

        assert_eq!(var_prefix.len(), 2);

        Ok(Image {
            input: image,
            name: Self::generate_calc_name(name),
            var_prefix: var_prefix.to_string(),
        })
    }

    /// Transform a string into one safe to use as a calculator variable name.
    fn generate_calc_name(s: &str) -> String {
        let mut chars = s.chars().filter(char::is_ascii_alphanumeric);

        // First character must be alphabetic, make it Z if not
        let head = chars
            .next()
            .map(|c| if !c.is_ascii_alphabetic() { 'Z' } else { c });

        // Recombine up to 8 chars, appending _ to pad if needed
        head.into_iter()
            .chain(chars)
            .chain(repeat('_'))
            .take(8)
            .collect()
    }

    /// Compute the palette for the loaded image.
    ///
    /// This stage can take a long time at high quality settings.
    pub fn quantize(self) -> QuantizedImage {
        let attrs = imagequant::Attributes::new();
        let bitmap = (&*self.input).as_rgba();
        let mut image = attrs
            .new_image_borrowed(
                bitmap,
                self.input.width() as usize,
                self.input.height() as usize,
                0.,
            )
            .expect("failed to construct imagequant image");

        let mut result = attrs
            .quantize(&mut image)
            .expect("failed to quantize image");
        let (palette, data) = result.remapped(&mut image).expect("failed to remap image");

        QuantizedImage {
            var_prefix: self.var_prefix,
            name: self.name,
            width: self.input.width(),
            height: self.input.height(),
            palette,
            data,
        }
    }
}

pub struct QuantizedImage {
    var_prefix: String,
    name: String,
    width: u32,
    height: u32,
    palette: Vec<RGBA>,
    data: Vec<u8>,
}

impl QuantizedImage {
    /// Return an iterator over image [`Tile`]s.
    pub fn tiles(&self) -> Tiles {
        Tiles {
            image: self,
            next: (0, 0),
        }
    }

    pub fn width_tiles(&self) -> u32 {
        self.width / Image::TILE_SIZE
    }

    pub fn height_tiles(&self) -> u32 {
        self.height / Image::TILE_SIZE
    }

    pub fn palette_appvar_name(&self) -> String {
        format!("HP{:2}0000", self.var_prefix)
    }

    pub fn write_palette_appvar<W: Write + Seek>(&self, out: W) -> IoResult<W> {
        let mut writer =
            tifiles::Writer::new(out, VariableType::AppVar, &self.palette_appvar_name(), true)?;

        // Header: signature, 8-character image name, 2-character var prefix
        // and index of last image tile.
        write!(
            writer,
            "HDPALV10{:8}{:2}{:03}{:03}",
            self.name,
            self.var_prefix,
            self.width_tiles() - 1,
            self.height_tiles() - 1,
        )?;

        // Palette data follows directly, little-endian RGB565
        for swatch in &self.palette {
            writer.write_all(&GRGB1555::from(swatch).to_le_bytes())?;
        }
        writer.close()
    }
}

/// Iterator over tiles in an image.
pub struct Tiles<'a> {
    image: &'a QuantizedImage,
    next: (u32, u32),
}

/// One slice of an overall image.
///
/// Tiles each get saved into one appvar on the calculator.
pub struct Tile<'a> {
    /// The location of this tile in the overall image. (0, 0) is top-left.
    index: (u32, u32),
    /// The name of the appvar generated from this tile.
    appvar_name: String,
    image: &'a QuantizedImage,
}

pub struct TileRows<'a> {
    tile: &'a Tile<'a>,
    y: u32,
}

impl<'a> Iterator for TileRows<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.y >= Image::TILE_SIZE {
            return None;
        }

        let row_start = (self.tile.index.1 * Image::TILE_SIZE + self.y) * self.tile.image.width;
        let row_offset = self.tile.index.0 * Image::TILE_SIZE;
        self.y += 1;

        let row_base = (row_start + row_offset) as usize;
        Some(&self.tile.image.data[row_base..row_base + Image::TILE_SIZE as usize])
    }
}

impl<'a> Tile<'a> {
    pub fn rows(&'a self) -> TileRows<'a> {
        TileRows { tile: self, y: 0 }
    }

    pub fn write_appvar<W: Write + Seek>(&self, out: W) -> IoResult<W> {
        let mut appvar = tifiles::Writer::new(out, VariableType::AppVar, &self.appvar_name, true)?;
        // Image data buffer so we can compress it
        let mut imgbuf =
            Vec::with_capacity(Image::TILE_SIZE as usize * Image::TILE_SIZE as usize + 2);

        // Image signature (not compressed)
        write!(appvar, "HDPICCV4{:8}", &self.image.name)?;
        // Image dimensions, always the tile size
        imgbuf.write_all(&[Image::TILE_SIZE as u8, Image::TILE_SIZE as u8])?;

        // Paletteized pixel data follows, row-major one byte per pixel
        for row in self.rows() {
            imgbuf.write_all(row)?;
        }
        debug_assert_eq!(
            imgbuf.capacity(),
            imgbuf.len(),
            "Initial buffer capacity was wrong"
        );

        // Then compress and write the compressed data
        let compressed = zx0::compress(&imgbuf);
        appvar.write_all(&compressed)?;
        appvar.close()
    }

    pub fn index(&self) -> (u32, u32) {
        self.index
    }

    pub fn appvar_name(&self) -> &str {
        &self.appvar_name
    }
}

impl<'a> Iterator for Tiles<'a> {
    type Item = Tile<'a>;

    fn next(&mut self) -> Option<Tile<'a>> {
        let (x, y) = self.next;
        if y == self.image.height_tiles() {
            None
        } else {
            self.next.0 += 1;
            if self.next.0 >= self.image.width_tiles() {
                self.next = (0, y + 1);
            }

            Some(Tile {
                index: (x, y),
                appvar_name: format!("{:2}{:03}{:03}", self.image.var_prefix, x, y),
                image: &self.image,
            })
        }
    }
}

/// 16-bit color format natively used by the calculator.
///
/// This is a version of RGB565, with 5 bits allocated to each of the red and green channels
/// and 6 in green. The LSb of green is placed at bit 15, followed on downward by red, green and
/// blue.
///
/// ```text
///  ┌──────────────────────────────┐
///  │ ┌────────────┐ ┌────────────┐▼┌────────────┐
///  │ │            │ │            │ │            │
/// G0 R4 R3 R2 R1 R0 G5 G4 G3 G2 G1 B4 B3 B2 B1 B0
/// 15 14 13 12 11 10  9  8  7  6  5  4  3  2  1  0
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct GRGB1555(u16);

/// Convert from 24-bit RGB.
impl From<&RGBA> for GRGB1555 {
    fn from(rgb: &RGBA) -> GRGB1555 {
        let r = (rgb.r as f32 / 255.0 * 0b1_1111 as f32).round() as u16;
        let g = (rgb.g as f32 / 255.0 * 0b11_1111 as f32).round() as u16;
        let b = (rgb.b as f32 / 255.0 * 0b1_1111 as f32).round() as u16;

        GRGB1555(((g & 0b00_0001) << 15) | (r << 10) | ((g & 0b11_1110) << 4) | b)
    }
}

impl std::ops::Deref for GRGB1555 {
    type Target = u16;

    fn deref(&self) -> &u16 {
        &self.0
    }
}

#[test]
fn rgb_conversion_is_correct() {
    assert_eq!(GRGB1555::from(&RGBA::new(0xff, 0xff, 0xff, 0xff)), GRGB1555(0xFFFF));
    assert_eq!(GRGB1555::from(&RGBA::new(0xff, 0, 0, 0xff)), GRGB1555(0x7c00));
    assert_eq!(GRGB1555::from(&RGBA::new(0, 0xff, 0, 0xff)), GRGB1555(0x83e0));
    assert_eq!(GRGB1555::from(&RGBA::new(0, 0, 0xff, 0xff)), GRGB1555(0x001f));
    assert_eq!(GRGB1555::from(&RGBA::new(0x5d, 0x37, 0x2c, 0xff)), GRGB1555(0x2CE5));
}
