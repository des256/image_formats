// image_formats::image
// by Desmond Germans, 2019

pub struct Image {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u32>,
}

impl Image {
    pub fn new(width: usize,height: usize) -> Image {
        Image {
            width: width,
            height: height,
            data: vec![0; width * height],
        }
    }
}
