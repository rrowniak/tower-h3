use std::io::{self, Write};

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct BmpFHEAD {
    pub bf_type: u16,
    pub bf_size: u32,
    pub bf_reserved: u32,
    pub bf_off_bits: u32,
}

impl BmpFHEAD {
    pub fn from(pixel_array_size: usize) -> Self {
        Self {
            bf_type: 0x4d42,
            bf_size: std::mem::size_of::<BmpFHEAD>() as u32
                + std::mem::size_of::<BmpIHEAD>() as u32
                + pixel_array_size as u32,
            bf_reserved: 0,
            bf_off_bits: std::mem::size_of::<BmpFHEAD>() as u32
                + std::mem::size_of::<BmpIHEAD>() as u32,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct BmpIHEAD {
    pub bi_size: u32,
    pub bi_width: i32,
    pub bi_height: i32,
    pub bi_planes: u16,
    pub bi_bit_count: u16,
    pub bi_compression: u32,
    pub bi_size_image: u32,
    pub bi_x_pels_per_meter: i32,
    pub bi_y_pels_per_meter: i32,
    pub bi_clr_used: u32,
    pub bi_clr_important: u32,
}

impl BmpIHEAD {
    pub fn from(width: usize, height: usize, pixel_array_size: usize) -> Self {
        Self {
            bi_size: std::mem::size_of::<BmpIHEAD>() as u32,
            bi_width: width as i32,
            bi_height: -(height as i32),
            bi_planes: 1,
            bi_bit_count: 24,
            bi_compression: 0,
            bi_size_image: pixel_array_size as u32,
            bi_x_pels_per_meter: 2835,
            bi_y_pels_per_meter: 2835,
            bi_clr_used: 0,
            bi_clr_important: 0,
        }
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub struct BMP<'a> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'a [u8],
}

impl<'a> BMP<'a> {
    pub fn from_mem(width: usize, height: usize, pixels: &'a [u8]) -> Self {
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn to_file(&self, filename: &std::path::Path) -> io::Result<()> {
        let row_size = (3 * self.width + 3) & !3; // Row size must be padded to 4 bytes
        let pixel_array_size = row_size * self.height;
        let file_header = BmpFHEAD::from(pixel_array_size);
        let info_header = BmpIHEAD::from(self.width, self.height, pixel_array_size);
        let mut file = std::fs::File::create(&filename)?;

        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                &file_header as *const BmpFHEAD as *const u8,
                std::mem::size_of::<BmpFHEAD>(),
            )
        };
        file.write_all(bytes)?;
        let bytes: &[u8] = unsafe {
            std::slice::from_raw_parts(
                &info_header as *const BmpIHEAD as *const u8,
                std::mem::size_of::<BmpIHEAD>(),
            )
        };
        file.write_all(bytes)?;
        // Write pixel data to file, row by row, with padding
        for y in 0..self.height {
            let start = (y * self.width * 3) as usize;
            let end = start + (self.width * 3) as usize;

            file.write_all(&self.pixels[start..end])?;

            // Add padding if needed
            let padding = vec![0u8; (row_size - self.width * 3) as usize];
            file.write_all(&padding)?;
        }
        // for y in 0..self.height {
        //     for x in 0..self.width {
        //         let pixel = self.pixels[(y * self.width + x) as usize];
        //         file.write_all(&[pixel.b, pixel.g, pixel.r])?;
        //     }
        //
        //     // Add padding if needed
        //     let padding = vec![0u8; (row_size - self.width * 3) as usize];
        //     file.write_all(&padding)?;
        // }
        Ok(())
    }
}
