// image_formats::image
// by Desmond Germans, 2019

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u32>,
}

impl Image {
    pub fn new(width: u32,height: u32) -> Image {
        Image {
            width: width,
            height: height,
            data: vec![0; (width * height) as usize],
        }
    }
}
