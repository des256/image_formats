// JPEG feature suite test
// by Desmond Germans, 2019

use std::io::{Read,Write};
use std::fs;
use std::fs::File;
extern crate glob;
use glob::glob;
use image_formats::jpeg;
use image_formats::bmp;

fn test(name: &str) {
    println!("testing {}...",name);
    let mut infile = File::open(&name).unwrap();
    let mut buffer = Vec::new();
    infile.read_to_end(&mut buffer).unwrap();
    match jpeg::test(&buffer) {
        Some((width,height)) => {
            println!("    Ok. Size {}x{}",width,height);
        },
        None => {
            println!("    Invalid JPEG");
        },
    }
}

fn load(name: &str) {
    println!("loading {}...",name);
    let mut infile = File::open(&name).unwrap();
    let mut buffer = Vec::new();
    infile.read_to_end(&mut buffer).unwrap();
    match jpeg::decode(&buffer) {
        Ok(image) => {
            let outname = (&name[0 .. name.len() - 4]).to_string() + ".bmp";
            match bmp::encode(&image) {
                Ok(value) => {
                    let mut outfile = File::create(&outname).unwrap();
                    outfile.write_all(&value).unwrap();
                },
                Err(msg) => {
                    println!("    Error: {}",msg);
                }
            };
        },
        Err(msg) => {
            println!("    Error: {}",msg);
        }
    }
}

fn remove_old_results() {
    for p in glob("../../../static/jpg/*.bmp").unwrap() {
        fs::remove_file(p.unwrap()).unwrap();
    }
}

fn test_test() {
    for p in glob("../../../static/jpg/*.jpg").unwrap() {
       test(p.unwrap().to_str().unwrap());
    }
}

fn test_load() {
    for p in glob("../../../static/jpg/*.jpg").unwrap() {
        load(p.unwrap().to_str().unwrap());
    }
}

pub fn main() {
    remove_old_results();
    //test_test();
    test_load();
    //load("../../../static/jpg/money.jpg");
}
