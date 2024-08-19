use resources::lod_reader;
use resources::pcx2bmp;
use std::env;

const USAGE: &str = "Usage: h3res [OPTIONS] [COMMAND] <input> <output>

Commands:
  show      Display the contents of the .lod file, including information about the embedded files.
  dump      Extract all files from the .lod archive and save them to the specified output directory.
  pcx2bmp   Convert a PCX file to BMP file.

Options:
  -h, --help     Show this help message and exit.

Examples:
  h3res show ./input/res.lod
      Display the list of files embedded in the res.lod archive.

  h3res dump ./input/res.lod ./my_dest_directory
      Extract all embedded files from res.lod and save them in ./my_dest_directory.

  h3res pcx2bmp ./input/res.pcx /my_dest_directory/res.bmp
  
Description:
  This tool allows you to interact with Heroes 3 resource files in the .lod format. You can either view the contents of the archive using the 'show' command, or extract the files using the 'dump' command.

For more information, contact the author.";

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("{USAGE}");
        std::process::exit(1);
    }
    match args[1].as_str() {
        "show" => {
            lod_reader::load_lod(std::path::Path::new(&args[2]), None, true);
        }
        "dump" => {
            if args.len() < 4 {
                panic!("Missing destination directory");
            }
            lod_reader::load_lod(
                std::path::Path::new(&args[2]),
                Some(std::path::Path::new(&args[3])),
                false,
            );
        }
        "pcx2bmp" => {
            if args.len() < 4 {
                panic!("Missing output bitmap file name");
            }
            pcx2bmp::convert_file(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
            );
        }
        s => {
            panic!("Unknown subcommand {s}");
        }
    }
}
