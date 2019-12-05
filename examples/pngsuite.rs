// PNG feature suite test
// by Desmond Germans, 2019

use std::io::{Read,Write};
use std::fs::File;
extern crate glob;
use glob::glob;
use image_formats::png;
use image_formats::bmp;

pub fn main() {

    for p in glob("../../../static/png/pngsuite/*.png").unwrap() {
        let name = p.unwrap();
        println!("{}...",name.display());
        let mut infile = File::open(&name).unwrap();
        let mut buffer = Vec::new();
        infile.read_to_end(&mut buffer).unwrap();
        match png::load(&buffer) {
            Ok(image) => {
                let mut bmpname = name.to_str().unwrap().to_string();  // just store the BMP at the same place the PNGs are located
                bmpname.pop();
                bmpname.pop();
                bmpname.pop();
                bmpname.push('b');
                bmpname.push('m');
                bmpname.push('p');
                match bmp::save(&image) {
                    Ok(value) => {
                        let mut outfile = File::create(&bmpname).unwrap();
                        outfile.write_all(&value).unwrap();
                    },
                    Err(msg) => {
                        println!("    Error: {}",msg);
                    }
                };
            },
            Err(msg) => {
                println!("    Error: {}",msg);
            },
        }
    }
}
