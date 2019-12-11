// PNG performance test
// by Desmond Germans, 2019

use std::io::{Read,Write};
use std::fs::File;
//use cpuprofiler::PROFILER;
use image_formats::png;
use image_formats::bmp;

pub fn main() {

    let mut infile = File::open("../../../static/png/huge.png").unwrap();
    let mut buffer = Vec::new();
    infile.read_to_end(&mut buffer).unwrap();
    //PROFILER.lock().unwrap().start("profile").expect("Couldn't start");
    let result = png::decode(&buffer);
    //PROFILER.lock().unwrap().stop().expect("Couldn't stop");
    match result {
        Ok(image) => {
            match bmp::encode(&image) {
                Ok(value) => {
                    let mut outfile = File::create("output.bmp").unwrap();
                    outfile.write_all(&value).unwrap();
                },
                Err(msg) => {
                    println!("    Error: {}",msg);
                }
            };
        },
        Err(msg) => {
            println!("error: {}",msg);
        },
    }
}
