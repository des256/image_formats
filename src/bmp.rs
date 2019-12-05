// image_formats::bmp
// by Desmond Germans, 2019

use crate::Image;

pub fn test(_src: &[u8]) -> Option<(u32,u32)> {
    None
}

pub fn load(_src: &[u8]) -> Result<Image,String> {
    Err("not implemented yet".to_string())
}

trait WriteTypes {
    fn push16(&mut self,d: u16);
    fn push16b(&mut self,d: u16);
    fn push32(&mut self,d: u32);
    fn push32b(&mut self,d: u32);
}

impl WriteTypes for Vec<u8> {
    fn push16(&mut self,d: u16) {
        self.push((d & 255) as u8);
        self.push((d >> 8) as u8);
    }
    fn push16b(&mut self,d: u16) {
        self.push((d >> 8) as u8);
        self.push((d & 255) as u8);
    }
    fn push32(&mut self,d: u32) {
        self.push((d & 255) as u8);
        self.push(((d >> 8) & 255) as u8);
        self.push(((d >> 16) & 255) as u8);
        self.push((d >> 24) as u8);
    }
    fn push32b(&mut self,d: u32) {
        self.push((d >> 24) as u8);
        self.push(((d >> 16) & 255) as u8);
        self.push(((d >> 8) & 255) as u8);
        self.push((d & 255) as u8);
    }
}

pub fn save(image: &Image) -> Result<Vec<u8>,String> {
    let headersize = 108;
    let stride = image.width * 4;
    let palettesize = 0;
    let bpp = 32;
    let compression = 3;
    let colors = 0;
    let redmask: u32 = 0x00FF0000;
    let greenmask: u32 = 0x0000FF00;
    let bluemask: u32 = 0x000000FF;
    let alphamask: u32 = 0xFF000000;

    let imagesize = stride * image.height;
    let offset = 14 + headersize + palettesize;
    let filesize = offset + imagesize;

    let mut dst: Vec<u8> = Vec::new();
    dst.push16b(0x424D);
    dst.push32(filesize);
    dst.push32(0);
    dst.push32(offset);
    dst.push32(headersize);
    dst.push32(image.width);
    dst.push32(-(image.height as i32) as u32);
    dst.push16(1);
    dst.push16(bpp);
    dst.push32(compression);
    dst.push32(imagesize);
    dst.push32(1);
    dst.push32(1);
    dst.push32(colors);
    dst.push32(colors);
    dst.push32(redmask);
    dst.push32(greenmask);
    dst.push32(bluemask);
    dst.push32(alphamask);
    dst.push32(0x57696E20);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    dst.push32(0);
    for y in 0..image.height {
        for x in 0..image.width {
            dst.push32(image.data[(y * image.width + x) as usize]);
        }
    }
    Ok(dst)
}
