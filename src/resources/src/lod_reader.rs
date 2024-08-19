use libz_sys::{uncompress, Z_BUF_ERROR, Z_DATA_ERROR, Z_MEM_ERROR, Z_OK};
use std::ffi::CStr;
use std::fs;
use std::path::Path;

const LOD_MAGIC: u32 = 0x00444f4c;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct H3File {
    name: [u8; 16],
    offset: u32,
    orig_size: u32,
    /// Seems like 1: h3c, 2: txt
    ftype: u32,
    compr_size: u32,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct Header {
    magic: u32,
    version: u32,
    files_num: u32,
    unknown: [u8; 80],
    h3file: [H3File; 10000],
}

pub fn load_lod(filename: &Path, dump_to: Option<&Path>, verbose: bool) {
    if !filename.exists() {
        panic!("load_lod: file {filename:?} does not exist");
    }

    if !filename.is_file() {
        panic!("load_lod: {filename:?} is not a file");
    }

    let bytes = match fs::read(&filename) {
        Ok(bytes) => bytes,
        Err(e) => panic!("load_lod: reading file {filename:?} failure: {e}"),
    };

    if std::mem::size_of::<Header>() > bytes.len() {
        panic!("load_lod: file {filename:?} seems to be smaller than LOD header. Corrupted file?");
    }

    let header: Header = unsafe { std::ptr::read(bytes.as_ptr() as *const _) };

    if header.magic != LOD_MAGIC {
        panic!("load_lod: file {filename:?} seems to be broken or not valid LOD format (magic check failed)");
    }

    let Header {
        magic,
        version,
        files_num,
        ..
    } = header;
    if verbose {
        println!("File {filename:?}, magic={magic}, version={version}, files_num={files_num}");
    }

    for (f, i) in header.h3file.iter().zip(0..files_num) {
        let c_str = CStr::from_bytes_until_nul(&f.name[..]).unwrap();
        let name = c_str.to_str().unwrap();
        if name.len() == 0 {
            break;
        }
        let H3File {
            offset,
            orig_size,
            ftype,
            compr_size,
            ..
        } = *f;
        if verbose {
            let comp_msg = if compr_size != 0 { "" } else { " UNCOMPRESSED" };
            println!("\t{i}  {name:16}\toffset{offset:9}\t\torig_size{orig_size:9}\ttype={ftype}\tcompr_size{compr_size:9}\t{comp_msg}");
        }

        // sanity check
        let from = offset as usize;
        let to = from + compr_size as usize;
        if from > to {
            panic!("load_lod: file {filename:?} contains invalid offset for {name} resource");
        }
        if to > bytes.len() {
            panic!("load_lod: file {filename:?} contains a reference pointing outside the file for {name}");
        }

        let fbytes = if compr_size != 0 {
            uncompress_h3file(&bytes[from..to], orig_size as usize)
        } else {
            bytes[from..from + orig_size as usize]
                .iter()
                .map(|b| *b)
                .collect()
        };
        // dumping this file to disk?
        if let Some(path) = dump_to {
            let dest_path = path.join(Path::new(name));
            if let Err(e) = fs::write(&dest_path, fbytes) {
                eprintln!("load_lod: can't write {dest_path:?}: {e}");
            }
        }
    }
}

fn uncompress_h3file(bytes: &[u8], orig_size: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(orig_size);
    out.resize(orig_size, 0);
    let mut dest_len = out.len() as u64;
    let compr_len = bytes.len() as u64;
    let res = unsafe { uncompress(out.as_mut_ptr(), &mut dest_len, bytes.as_ptr(), compr_len) };
    match res {
        Z_OK =>{}
        Z_BUF_ERROR => panic!("load_h3file: the buffer dest {orig_size} was not large enough to hold the uncompressed data."),
        Z_MEM_ERROR => panic!("load_h3file: insufficient memory"),
        Z_DATA_ERROR => panic!("load_h3file: the compressed data (referenced by source) was corrupted."),
        _ => panic!("load_h3file: unknown zlib error"),
    }
    out
}
