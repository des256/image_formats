// image_formats::tga
// by Desmond Germans, 2019

use crate::Image;

pub fn test(_src: &[u8]) -> Option<(u32,u32)> {
	None
}

pub fn load(_src: &[u8]) -> Result<Image,String> {
    Err("not implemented yet".to_string())
}

pub fn save(_image: &Image) -> Result<Vec<u8>,String> {
    Err("not implemented yet".to_string())
}
