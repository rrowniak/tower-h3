use std::io::{self, Cursor, Read};
extern crate libz_sys as zlib;
use std::ptr;

const GZIP_MAGIC: u32 = 0x00088B1F;

pub struct BinaryDataReader {
    cursor: Cursor<Vec<u8>>,
}

impl BinaryDataReader {
    pub fn new(data: Vec<u8>) -> Self {
        BinaryDataReader {
            cursor: Cursor::new(data),
        }
    }

    pub fn new_possibly_gzip(data: Vec<u8>) -> Result<Self, io::Error> {
        if data.len() >= 4 {
            let magic = (data[3] as u32) << 24
                | (data[2] as u32) << 16
                | (data[1] as u32) << 8
                | data[0] as u32;
            if magic == GZIP_MAGIC {
                // this is gzip archive
                return Ok(Self::new(decompress(&data)?));
            }
        }
        Ok(Self::new(data))
    }

    // Read a single u8
    pub fn read_u8(&mut self) -> io::Result<u8> {
        let mut buffer = [0u8; 1];
        self.cursor.read_exact(&mut buffer)?;
        Ok(buffer[0])
    }

    // Read a single u16 assuming little-endian byte order
    pub fn read_u16_le(&mut self) -> io::Result<u16> {
        let mut buffer = [0u8; 2];
        self.cursor.read_exact(&mut buffer)?;
        Ok((buffer[1] as u16) << 8 | buffer[0] as u16)
    }

    // Read a single u32 assuming little-endian byte order
    pub fn read_u32_le(&mut self) -> io::Result<u32> {
        let mut buffer = [0u8; 4];
        self.cursor.read_exact(&mut buffer)?;
        Ok((buffer[3] as u32) << 24
            | (buffer[2] as u32) << 16
            | (buffer[1] as u32) << 8
            | buffer[0] as u32)
    }

    // Read a single f32 assuming little-endian byte order
    pub fn read_f32_le(&mut self) -> io::Result<f32> {
        let value = self.read_u32_le()?;
        Ok(f32::from_bits(value))
    }

    // Read a single u64 assuming little-endian byte order
    pub fn read_u64_le(&mut self) -> io::Result<u64> {
        let mut buffer = [0u8; 8];
        self.cursor.read_exact(&mut buffer)?;
        Ok((buffer[7] as u64) << 56
            | (buffer[6] as u64) << 48
            | (buffer[5] as u64) << 40
            | (buffer[4] as u64) << 32
            | (buffer[3] as u64) << 24
            | (buffer[2] as u64) << 16
            | (buffer[1] as u64) << 8
            | buffer[0] as u64)
    }
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, io::Error> {
    unsafe {
        // Initialize z_stream
        let mut stream = zlib::z_stream {
            next_in: data.as_ptr() as *mut u8, // Input data pointer
            avail_in: data.len() as u32,       // Available input data size
            next_out: ptr::null_mut(),         // Will be set later for output
            avail_out: 0,                      // Available output size
            // ..zlib::z_stream::default()        // Fill remaining fields with default values
            adler: 0,
            data_type: 0,
            msg: std::ptr::null_mut(),
            opaque: std::ptr::null_mut(),
            reserved: 0,
            state: std::ptr::null_mut(),
            total_in: 0,
            total_out: 0,
            // zalloc: std::mem::uninitialized(),
            #[allow(invalid_value)]
            zalloc: std::mem::MaybeUninit::zeroed().assume_init(),
            #[allow(invalid_value)]
            zfree: std::mem::MaybeUninit::zeroed().assume_init(),
        };

        // Allocate an initial buffer for the output
        let mut output: Vec<u8> = Vec::with_capacity(1024);

        let mut wbits = 15;
        wbits += 16; // if zlib
        let ret = zlib::inflateInit2_(
            &mut stream,
            wbits,
            libz_sys::zlibVersion(),
            std::mem::size_of::<zlib::z_stream>() as i32,
        );

        if ret != zlib::Z_OK {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to initialize zlib",
            ));
        }

        // Decompression loop
        loop {
            // Extend the output buffer when needed
            output.reserve(1024);
            let output_ptr = output.as_mut_ptr().add(output.len());
            let available_output = output.capacity() - output.len();

            // Set the output buffer in the z_stream
            stream.next_out = output_ptr;
            stream.avail_out = available_output as u32;

            // Perform the inflation (decompression)
            let ret = zlib::inflate(&mut stream, zlib::Z_NO_FLUSH);

            // Update the length of the output based on how much data was written
            let written = available_output - stream.avail_out as usize;
            output.set_len(output.len() + written);

            // Check the return value of `inflate` to see if we're done
            match ret {
                zlib::Z_OK | zlib::Z_BUF_ERROR => continue, // Keep decompressing
                zlib::Z_STREAM_END => break,                // Done
                zlib::Z_DATA_ERROR => {
                    // If we hit an error, clean up and return an error
                    zlib::inflateEnd(&mut stream);
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "Decompression failed: Z_DATA_ERROR",
                    ));
                }

                _ => {
                    // If we hit an error, clean up and return an error
                    zlib::inflateEnd(&mut stream);
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Decompression failed: {ret}").as_str(),
                    ));
                }
            }
        }

        // Clean up the decompression state
        zlib::inflateEnd(&mut stream);

        Ok(output)
    }
}
