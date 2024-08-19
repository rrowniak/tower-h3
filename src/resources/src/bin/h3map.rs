use resources::map_reader;
use std::env;

const USAGE: &str = "Usage: h3map [OPTIONS] [COMMAND] <input> <output>

Commands:
  show      Display some information about the .h3m file.

Options:
  -h, --help     Show this help message and exit.

Examples:
  h3map show ./input/res.h3m
 
Description:
  This tool allows you to interact with Heroes 3 map files in the .h3m format.

For more information, contact the author.";

fn main() {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        println!("{USAGE}");
        std::process::exit(1);
    }
    match args[1].as_str() {
        "show" => {
            if let Err(e) = map_reader::load_h3m(std::path::Path::new(&args[2])) {
                panic!("Cant load map {}: {e}", &args[2]);
            }
        }
        s => {
            panic!("Unknown subcommand {s}");
        }
    }
}
