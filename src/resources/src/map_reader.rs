use crate::map_structs::*;
use crate::reader;
use std::io;
use std::path::Path;

pub fn load_h3m(filename: &Path) -> io::Result<()> {
    if !filename.exists() {
        panic!("load_h3m: file {filename:?} does not exist");
    }

    if !filename.is_file() {
        panic!("load_h3m: {filename:?} is not a file");
    }
    let mut reader = reader::BinaryDataReader::new_possibly_gzip(std::fs::read(&filename)?)?;
    let map_format = reader.read_u32_le()?;
    let map_format = Format::from(map_format).expect("dddd");
    println!("{}", map_format.nice_str());
    Ok(())
}
