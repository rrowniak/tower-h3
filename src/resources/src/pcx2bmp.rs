use crate::bmp;
use std::path::Path;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct PCX {
    bitmap_size: u32,
    width: u32,
    height: u32,
}

pub fn convert_file(src: &Path, dst: &Path) {
    if !src.exists() {
        panic!("pcx2bmp: file {src:?} does not exist");
    }
    if !src.is_file() {
        panic!("pcx2bmp: {src:?} is not a file");
    }

    let bytes = match std::fs::read(&src) {
        Ok(bytes) => bytes,
        Err(e) => panic!("pcx2bmp: reading file {src:?} failure: {e}"),
    };

    if std::mem::size_of::<PCX>() > bytes.len() {
        panic!("pcx2bmp: file {src:?} seems to be smaller than PCX header. Corrupted file?");
    }

    let header: PCX = unsafe { std::ptr::read(bytes.as_ptr() as *const _) };
    let PCX {
        bitmap_size,
        width,
        height,
    } = header;
    println!("{src:?}: {width} x {height}, bitmap size {bitmap_size}");

    let pcxh_size = std::mem::size_of::<PCX>();
    let got_pixels = bytes.len() - pcxh_size;
    let exp_pixels_8b = (width * height + 3 * 256) as usize;
    let exp_pixels_24b = (3 * width * height) as usize;
    match got_pixels {
        p if p == exp_pixels_8b => {
            let mut pixels = Vec::<u8>::with_capacity((3 * width * height) as usize);
            for i in bytes[pcxh_size..bytes.len() - 256 * 3].iter() {
                let (r, g, b) = pix_from_pal(&bytes[bytes.len() - 256 * 3..], *i);
                pixels.push(b); // b
                pixels.push(g); // g
                pixels.push(r); // r
            }

            let bmp = bmp::BMP::from_mem(width as usize, height as usize, &pixels);

            if let Err(e) = bmp.to_file(dst) {
                panic!("pcx2bmp: writing to {dst:?} error: {e}");
            }
        }
        p if p == exp_pixels_24b => {
            let bmp = bmp::BMP::from_mem(width as usize, height as usize, &bytes[pcxh_size..]);

            if let Err(e) = bmp.to_file(dst) {
                panic!("pcx2bmp: writing to {dst:?} error: {e}");
            }
        }
        exp_pixels => {
            if exp_pixels != got_pixels {
                panic!("pcx2bmp: unexpected pixel count, got {got_pixels}, expected {exp_pixels}");
            }
        }
    }
}

fn pix_from_pal(palette: &[u8], index: u8) -> (u8, u8, u8) {
    if palette.len() != 256 * 3 {
        let pallen = palette.len();
        panic!("Palette size is {pallen}");
    }
    let index = index as usize;
    let index = index * 3;
    (palette[index], palette[index + 1], palette[index + 2])
}
