// image_formats::bmp
// by Desmond Germans, 2019

use crate::Image;

const TYPE_C1: u16 = 0x0001;
const TYPE_C2: u16 = 0x0002;
const TYPE_C4: u16 = 0x0004;
const TYPE_C4_RLE: u16 = 0x0204;
const TYPE_C8: u16 = 0x0008;
const TYPE_C8_RLE: u16 = 0x0108;
const TYPE_A1RGB5: u16 = 0x0010;
const TYPE_B16: u16 = 0x0310;
const TYPE_RGB8: u16 = 0x0018;
const TYPE_ARGB8: u16 = 0x0020;
const TYPE_B32: u16 = 0x0320;

fn from_le16(src: &[u8]) -> u16 {
    ((src[1] as u16) << 8) | (src[0] as u16)
}

fn from_le32(src: &[u8]) -> u32 {
    ((src[3] as u32) << 24) | ((src[2] as u32) << 16) | ((src[1] as u32) << 8) | (src[0] as u32)
}

struct Component {
    mask: u32,
    shift: u32,
    size: u32,
}

impl Component {
    pub fn new(mask: u32) -> Component {
        let mut shift = 0;
        let mut size = 0;
        let mut last_bit = false;
        let mut shift_found = false;
        let mut size_found = false;
        for i in 0..32 {
            let bit = (mask & (1 << i)) != 0;
            if bit != last_bit {
                if bit {
                    if !shift_found {
                        shift = i;
                        shift_found = true;
                    }
                } else {
                    size = i - shift;
                    size_found = true;
                    break;
                }
                last_bit = bit;
            }
        }
        if !size_found {
            size = 32 - shift;
        }
        Component {
            mask: mask,
            shift: shift,
            size: size,
        }
    }

    pub fn get(&self,c: u32,def: u8) -> u8 {
        if self.size == 0 {
            return def;
        }
        let d = (c & self.mask) >> self.shift;
        match self.size {
            1 => if d != 0 { 255 } else { 0 },
            2 => ((d << 6) | (d << 4) | (d << 2) | d) as u8,
            3 => ((d << 5) | (d << 2) | (d >> 1)) as u8,
            4 => ((d << 4) | d) as u8,
            5 => ((d << 3) | (d >> 2)) as u8,
            6 => ((d << 2) | (d >> 4)) as u8,
            7 => ((d << 1) | (d >> 6)) as u8,
            _ => (d >> (self.size - 8)) as u8,
        }
    }
}

pub fn decode(dst: &mut [u32],src: &[u8],width: u32,height: u32,bottom_up: bool,itype: u16,palette: &[u32; 256],redmask: u32,greenmask: u32,bluemask: u32,alphamask: u32) {
    let red = Component::new(redmask);
    let green = Component::new(greenmask);
    let blue = Component::new(bluemask);
    let alpha = Component::new(alphamask);
    let mut sp = 0usize;
    let mut y = 0u32;
    let mut dy = 1i32;
    if bottom_up {
        y = height - 1;
        dy = -1;
    }
    let mut line = width * y;
    let dline = width as i32 * dy;
    match itype {
        TYPE_C1 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width / 8 {
                    let d = src[sp];
                    sp += 1;
                    for i in 0..8 {
                        dst[dp] = palette[((d >> (7 - i)) & 1) as usize];
                        dp += 1;
                    }
                }
                if (width & 7) != 0 {
                    let d = src[sp];
                    sp += 1;
                    for i in 0..(width & 7) {
                        dst[dp] = palette[((d >> (7 - i)) & 1) as usize];
                        dp += 1;
                    }
                }
                let rest = ((width + 7) / 8) & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_C2 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width / 4 {
                    let d = src[sp];
                    sp += 1;
                    for i in 0..4 {
                        dst[dp] = palette[((d >> (6 - 2 * i)) & 3) as usize];
                        dp += 1;
                    }
                }
                if (width & 3) != 0 {
                    let d = src[sp];
                    sp += 1;
                    for i in 0..(width & 3) {
                        dst[dp] = palette[((d >> (6 - 2 * i)) & 3) as usize];
                        dp += 1;
                    }
                }
                let rest = ((width + 3) / 4) & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_C4 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width / 2 {
                    let d = src[sp];
                    sp += 1;
                    for i in 0..2 {
                        dst[dp] = palette[((d >> (4 - 4 * i)) & 15) as usize];
                        dp += 1;
                    }
                }
                if (width & 1) != 0 {
                    let d = src[sp];
                    sp += 1;
                    dst[dp] = palette[(d & 15) as usize];
                }
                let rest = ((width + 1) / 2) & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_C4_RLE => {
            let mut x = 0u32;
            while sp < src.len() {
                let code: u16 = from_le16(&src[sp..sp+2]);
                sp += 2;
                match code {
                    0x0000 => {
                        x = 0;
                        y = ((y as i32) + dy) as u32;
                    },
                    0x0100 => {
                        break;
                    },
                    0x0200 => {
                        x += src[sp] as u32;
                        y = ((y as i32) + (src[sp + 1] as i32) * dy) as u32;
                        sp += 2;
                    },
                    _ => {
                        if (code & 255) != 0 {
                            let count = code & 255;
                            if x + count as u32 > width {
                                break;
                            }
                            let c0 = palette[(code >> 12) as usize];
                            let c1 = palette[((code >> 8) & 15) as usize];
                            for _i in 0..count / 2 {
                                dst[(y * width + x) as usize] = c0;
                                dst[(y * width + x + 1) as usize] = c1;
                                x += 2;
                            }
                            if (count & 1) != 0 {
                                dst[(y * width + x) as usize] = c0;
                                x += 1;
                            }
                        }
                        else {
                            let count = code >> 8;
                            if x + count as u32 > width {
                                break;
                            }
                            for _i in 0..count / 4 {
                                let c = from_le16(&src[sp..sp+2]);
                                sp += 2;
                                dst[(y * width + x) as usize] = palette[((c >> 4) & 15) as usize];
                                dst[(y * width + x + 1) as usize] = palette[(c & 15) as usize];
                                dst[(y * width + x + 2) as usize] = palette[(c >> 12) as usize];
                                dst[(y * width + x + 3) as usize] = palette[((c >> 8) & 15) as usize];
                                x += 4;
                            }
                            if (count & 3) != 0 {
                                let c = from_le16(&src[sp..sp+2]);
                                sp += 2;
                                if (count & 3) >= 1 {
                                    dst[(y * width + x) as usize] = palette[((c >> 4) & 15) as usize];
                                    x += 1;
                                }
                                if (count & 3) >= 2 {
                                    dst[(y * width + x) as usize] = palette[(c & 15) as usize];
                                    x += 1;
                                }
                                if (count & 3) >= 3 {
                                    dst[(y * width + x) as usize] = palette[(c >> 12) as usize];
                                    x += 1;
                                }
                            }
                        }
                    }
                }
            }
        },
        TYPE_C8 => {
            for _l in 0..height {
                let mut dp = line;
                for _x in 0..width {
                    dst[dp as usize] = palette[src[sp] as usize];
                    sp += 1;
                    dp += 1;
                }
                let rest = width & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_C8_RLE => {
            let mut x = 0u32;
            while sp < src.len() {
                let code: u16 = from_le16(&src[sp..sp+2]);
                sp += 2;
                match code {
                    0x0000 => {
                        x = 0;
                        y = ((y as i32) + dy) as u32;
                    },
                    0x0100 => {
                        break;
                    },
                    0x0200 => {
                        x += src[sp] as u32;
                        y = ((y as i32) + (src[sp + 1] as i32) * dy) as u32;
                        sp += 2;
                    },
                    _ => {
                        if (code & 255) != 0 {
                            let count = code & 255;
                            if x + count as u32 > width {
                                break;
                            }
                            let c = palette[(code >> 8) as usize];
                            for _i in 0..count {
                                dst[(y * width + x) as usize] = c;
                                x += 1;
                            }
                        }
                        else {
                            let count = code >> 8;
                            if x + count as u32 > width {
                                break;
                            }
                            for _i in 0..count / 2 {
                                let c = from_le16(&src[sp..sp+2]);
                                sp += 2;
                                dst[(y * width + x) as usize] = palette[(c & 255) as usize];
                                dst[(y * width + x + 1) as usize] = palette[(c >> 8) as usize];
                                x += 2;
                            }
                            if (count & 1) != 0 {
                                let c = from_le16(&src[sp..sp+2]);
                                sp += 2;
                                dst[(y * width + x) as usize] = palette[(c & 255) as usize];
                                x += 1;
                            }
                        }
                    },
                }
			}
        },
        TYPE_A1RGB5 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width {
                    let d = from_le16(&src[sp..sp+2]);
                    sp += 2;
                    let mut r = (d >> 10) & 31;
                    let mut g = (d >> 5) & 31;
                    let mut b = d & 31;
                    let a = if alphamask == 0 { 255 } else if (d & 0x8000) != 0 { 255 } else { 0 };
                    r = (r << 3) | (r >> 2);
                    g = (g << 3) | (g >> 2);
                    b = (b << 3) | (b >> 2);
                    //println!("{},{}: {:04X} - a{} r{} g{} b{}",x,line,d,a,r,g,b);
                    dst[dp] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    dp += 1;
                }
                let rest = (width * 2) & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_B16 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width {
                    let d = from_le16(&src[sp..sp+2]) as u32;
                    sp += 2;
                    let r = red.get(d,0);
                    let g = green.get(d,0);
                    let b = blue.get(d,0);
                    let a = if alphamask == 0 { 255 } else { alpha.get(d,255) };
                    dst[dp] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    dp += 1;
                }
                let rest = (width * 2) & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_RGB8 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width {
                    let b = src[sp];
                    let g = src[sp + 1];
                    let r = src[sp + 2];
                    sp += 3;
                    dst[dp] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    dp += 1;
                }
                let rest = (width * 3) & 3;
                if rest > 0 {
                    sp += (4 - rest) as usize;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_ARGB8 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width {
                    let d = from_le32(&src[sp..sp+4]);
                    sp += 4;
                    let r = (d >> 16) & 255;
                    let g = (d >> 8) & 255;
                    let b = d & 255;
                    let a = if alphamask == 0 { 255 } else { d >> 24 };
                    dst[dp] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    dp += 1;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        TYPE_B32 => {
            for _l in 0..height {
                let mut dp = line as usize;
                for _x in 0..width {
                    let d = from_le32(&src[sp..sp+4]);
                    sp += 4;
                    let r = red.get(d,0);
                    let g = green.get(d,0);
                    let b = blue.get(d,0);
                    let a = if alphamask == 0 { 255 } else { alpha.get(d,255) };
                    dst[dp] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    dp += 1;
                }
                line = (line as i32 + dline) as u32;
            }
        },
        _ => { },
    }
}

pub fn test(src: &[u8]) -> Option<(u32,u32)> {
    let tag = from_le16(&src[0..2]);
    if (tag == 0x4D42) ||   // BM (Windows BMP)
        (tag == 0x4142) ||  // BA (OS/2 bitmap)
        (tag == 0x4943) ||  // CI (OS/2 color icon)
        (tag == 0x5043) ||  // CP (OS/2 color pointer) 
        (tag == 0x4349) ||  // IC (OS/2 icon)
        (tag == 0x5450) {    // PT (OS/2 pointer)
        let filesize = from_le32(&src[2..6]);
        let offset = from_le32(&src[10..14]);
        let headersize = from_le32(&src[14..18]);
        if (headersize > filesize) || (offset > filesize) || (headersize > offset) || (filesize != src.len() as u32) {
            return None;
        }
        if (headersize != 12) &&
           (headersize != 40) &&
           (headersize != 52) &&
           (headersize != 56) &&
           (headersize != 108) &&
           (headersize != 124) {
            return None;
        }
        if headersize == 12 {
            let width = from_le16(&src[18..20]);
            let mut height = from_le16(&src[20..22]);
            if (height as i16) < 0 {
                height = -(height as i16) as u16;
            }
            if (width > 32768) || (height > 32768) || (width == 0) || (height == 0) {
                return None;
            }
            let planes = from_le16(&src[22..24]);
            let itype = from_le16(&src[24..26]);
            if planes != 1 {
                return None;
            }
            let mut line = match itype {
                TYPE_C1 => (width + 7) / 8,
                TYPE_C4 => (width + 1) / 2,
                TYPE_C8 => width,
                TYPE_RGB8 => width * 3,
                _ => { return None; },
            };
            let rest = line & 3;
            if rest > 0 {
                line += 4 - rest;
            }
            if offset as usize + (height * line) as usize > src.len() {
                return None;
            }
            return Some((width as u32,height as u32));
        }
        else {
            let width = from_le32(&src[18..22]);
            let mut height = from_le32(&src[22..26]);
            if (height as i32) < 0 {
                height = -(height as i32) as u32;
            }
            if (width > 32768) || (height > 32768) || (width == 0) || (height == 0) {
                return None;
            }
            //let planes = from_le16(&src[26..28]);
            let bpp = from_le16(&src[28..30]);
            let compression = from_le32(&src[30..34]) as u16;
            let itype = (compression << 8) | bpp;
            let mut line = match itype {
                TYPE_C1 => (width + 7) / 8,
                TYPE_C2 => (width + 3) / 4,
                TYPE_C4 => (width + 1) / 2,
                TYPE_C4_RLE => 0,
                TYPE_C8 => width,
                TYPE_C8_RLE => 0,
                TYPE_A1RGB5 | TYPE_B16 => width * 2,
                TYPE_RGB8 => width * 3,
                TYPE_ARGB8 | TYPE_B32 => width * 4,
                _ => { return None; },
            };
            let rest = line & 3;
            if rest > 0 {
                line += 4 - rest;
            }
            if (line != 0) && (offset as usize + (height * line) as usize > src.len()) {
                return None;
            }
            return Some((width,height));
        }
    }
    None
}

pub fn load(src: &[u8]) -> Result<Image,String> {
    let tag = from_le16(&src[0..2]);
    if (tag != 0x4D42) &&
        (tag != 0x4142) &&
        (tag != 0x4943) &&
        (tag != 0x5043) && 
        (tag != 0x4349) &&
        (tag != 0x5450) {
        return Err("Invalid BMP".to_string());
    }
    let filesize = from_le32(&src[2..6]);
    let offset = from_le32(&src[10..14]);
    let headersize = from_le32(&src[14..18]);
    if (headersize > filesize) || (offset > filesize) || (headersize > offset) || (filesize != src.len() as u32) {
        return Err("Invalid BMP".to_string());
    }
    if (headersize != 12) &&
        (headersize != 40) &&
        (headersize != 52) &&
        (headersize != 56) &&
        (headersize != 108) &&
        (headersize != 124) {
        return Err("Invalid BMP".to_string());
    }
    #[allow(unused_assignments)]
    let mut width = 0u32;
    #[allow(unused_assignments)]
    let mut height = 0u32;
    let mut bottom_up = true;
    #[allow(unused_assignments)]
    let mut itype = 0u16;
    let mut palette = [0u32; 256];
    let mut redmask = 0u32;
    let mut greenmask = 0u32;
    let mut bluemask = 0u32;
    let mut alphamask = 0u32;
    if headersize == 12 {
        width = from_le16(&src[18..20]) as u32;
        let pheight = from_le16(&src[20..22]) as i16;
        height = if pheight < 0 { bottom_up = false; -pheight as u32 } else { pheight as u32 };
        if (width > 32768) || (height > 32768) || (width == 0) || (height == 0) {
            return Err("Invalid BMP".to_string());
        }
        let planes = from_le16(&src[22..24]);
        itype = from_le16(&src[24..26]);
        if planes != 1 {
            return Err("Invalid BMP".to_string());
        }
        let mut line = match itype {
            TYPE_C1 => (width + 7) / 8,
            TYPE_C4 => (width + 1) / 2,
            TYPE_C8 => width,
            TYPE_RGB8 => width * 3,
            _ => { return Err("Invalid BMP".to_string()); },
        };
        let rest = line & 3;
        if rest > 0 {
            line += 4 - rest;
        }
        if offset as usize + (height * line) as usize > src.len() {
            return Err("Invalid BMP".to_string());
        }
    }
    else {
        width = from_le32(&src[18..22]);
        let pheight = from_le32(&src[22..26]) as i32;
        height = if pheight < 0 { bottom_up = false; -pheight as u32 } else { pheight as u32 };
        if (width > 32768) || (height > 32768) || (width == 0) || (height == 0) {
            return Err("Invalid BMP".to_string());
        }
        //let planes = from_le16(&src[26..28]);
        let bpp = from_le16(&src[28..30]);
        let compression = from_le32(&src[30..34]) as u16;
        itype = (compression << 8) | bpp;
        let mut line = match itype {
            TYPE_C1 => (width + 7) / 8,
            TYPE_C2 => (width + 3) / 4,
            TYPE_C4 => (width + 1) / 2,
            TYPE_C4_RLE => 0,
            TYPE_C8 => width,
            TYPE_C8_RLE => 0,
            TYPE_A1RGB5 | TYPE_B16 => width * 2,
            TYPE_RGB8 => width * 3,
            TYPE_ARGB8 | TYPE_B32 => width * 4,
            _ => { return Err("Invalid BMP".to_string()); },
        };
        let rest = line & 3;
        if rest > 0 {
            line += 4 - rest;
        }
        if (line != 0) && (offset as usize + (height * line) as usize > src.len()) {
            return Err("Invalid BMP".to_string());
        }
        let imagesize = from_le32(&src[34..38]);
        if (compression == 0) && (imagesize > filesize - offset) {
            return Err("Invalid BMP".to_string());
        }
        // 38..46: resolution
        let mut colors = from_le32(&src[46..50]);
        // 50..54: important colors
        match itype {
            TYPE_C1 | TYPE_C2 | TYPE_C4 | TYPE_C4_RLE | TYPE_C8 | TYPE_C8_RLE => {
                if colors == 0 {
                    colors = 1 << bpp;
                } else if colors > 256 {
                    return Err("Invalid BMP".to_string());
                }
                for i in 0..colors {
                    let sp = (14 + headersize + i * 4) as usize;
                    let b = src[sp];
                    let g = src[sp + 1];
                    let r = src[sp + 2];
                    palette[i as usize] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }
            },
            TYPE_B16 | TYPE_B32 => {
                redmask = from_le32(&src[54..58]);
                greenmask = from_le32(&src[58..62]);
                bluemask = from_le32(&src[62..66]);
                if (headersize >= 56) || ((offset - headersize - 14) >= 16) {
                    alphamask = from_le32(&src[66..70]);
                }
            },
            TYPE_A1RGB5 => {
                alphamask = if headersize < 56 { 0 } else { 0x8000 };
            },
            TYPE_ARGB8 => {
                alphamask = if headersize < 56 { 0 } else { 0xFF000000 };
            }
            _ => { },
        }
    }
    let mut image = Image::new(width,height);
    decode(&mut image.data,&src[offset as usize..],width,height,bottom_up,itype,&palette,redmask,greenmask,bluemask,alphamask);
    Ok(image)
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
    dst.push16b(0x424D);  // 0
    dst.push32(filesize);  // 2
    dst.push32(0);  // 6
    dst.push32(offset);  // 10
    dst.push32(headersize);  // 14
    dst.push32(image.width);  // 18
    dst.push32(-(image.height as i32) as u32);  // 22
    dst.push16(1);  // 26
    dst.push16(bpp);  // 28
    dst.push32(compression);  // 30
    dst.push32(imagesize);  // 34
    dst.push32(1);  // 38
    dst.push32(1);  // 42
    dst.push32(colors);  // 46
    dst.push32(colors);  // 50
    dst.push32(redmask);  // 54
    dst.push32(greenmask);  // 58
    dst.push32(bluemask);  // 62
    dst.push32(alphamask);  // 66
    dst.push32(0x57696E20);  // 70
    dst.push32(0);  // 74
    dst.push32(0);  // 78
    dst.push32(0);  // 82
    dst.push32(0);  // 86
    dst.push32(0);  // 90
    dst.push32(0);  // 94
    dst.push32(0);  // 98
    dst.push32(0);  // 102
    dst.push32(0);  // 106
    dst.push32(0);  // 110
    dst.push32(0);  // 114
    dst.push32(0);  // 118
    for y in 0..image.height {
        for x in 0..image.width {
            dst.push32(image.data[(y * image.width + x) as usize]);  // 122..
        }
    }
    Ok(dst)
}


/*
class component
{
public:
	uint shift;
	uint32 mask;
	uint size;

	component() : shift(0),mask(0),size(0)
	{
	};

	component(uint32 amask) : shift(0),mask(0),size(0)
	{
		set(amask);
	};

	void set(uint32 amask)
	{
		shift = 0;
		mask = amask;
		size = 0;
		for(uint i = 0; i < 32; i++)
			if(mask & (1 << i))
			{
				shift = i;
				break;
			}
		uint i;
		for(i = shift; i < 32; i++)
			if(!(mask & (1 << i)))
				break;
		size = i - shift;
	};

	uint8 get(uint c,uint8 def)
	{
		if(!size)
			return def;
		uint d = (c & mask) >> shift;
		switch(size)
		{
			case 1: return d?0xFF:0x00;
			case 2: return (d << 6) | (d << 4) | (d << 2) | d;
			case 3: return (d << 5) | (d << 2) | (d >> 1);
			case 4: return (d << 4) | d;
			case 5: return (d << 3) | (d >> 2);
			case 6: return (d << 2) | (d >> 4);
			case 7: return (d << 1) | (d >> 6);
			default: return d >> (size - 8);
		}
		return 0;
	};
};
void Decode(Image::pixels& pix,const uint8*& sp,uint type,uint32* palette,bool reverse,component& red,component& green,component& blue,component& alpha)
{
	intxy siz = pix.size();
	bool eoi = false;
	int x = 0,y = 0;
	int dx = 1,dy = 1;
	int endy = siz.y;
	if(reverse)
	{
		y = siz.y - 1;
		dy = -1;
		endy = -1;
	}
	switch(type)
	{
		case TYPE_C1:
			for(; y != endy; y += dy)
			{
				int x;
				for(x = 0; x < siz.x / 8; x++)
				{
					uint8 d = get8(sp);
					for(int i = 0; i < 8; i++)
						pix.put8un(intxy(x * 8 + i,y),(d & (1 << i))?palette[1]:palette[0]);
				}
				if(siz.x & 7)
				{
					uint8 d = get8(sp);
					for(int i = 0; i < (siz.x & 7); i++)
						pix.put8un(intxy(x * 8 + i,y),(d & (1 << i))?palette[1]:palette[0]);
				}
			}
			break;

		case TYPE_C2:
			for(; y != endy; y += dy)
			{
				int x;
				for(x = 0; x < siz.x / 4; x++)
				{
					uint8 d = get8(sp);
					for(int i = 0; i < 4; i++)
						pix.put8un(intxy(x * 4 + i,y),palette[(d >> (6 - 2 * i)) & 3]);
				}
				if(siz.x & 3)
				{
					uint8 d = get8(sp);
					for(int i = 0; i < (siz.x & 3); i++)
						pix.put8un(intxy(x * 4 + i,y),palette[(d >> (6 - 2 * i)) & 3]);
				}
			}
			break;

		case TYPE_C4:
			for(; y != endy; y += dy)
			{
				int x;
				for(x = 0; x < siz.x / 2; x++)
				{
					uint8 d = get8(sp);
					for(int i = 0; i < 2; i++)
						pix.put8un(intxy(x * 2 + i,y),palette[(d >> (4 - 4 * i)) & 15]);
				}
				if(siz.x & 1)
				{
					uint8 d = get8(sp);
					pix.put8un(intxy(x * 2,y),palette[d & 15]);
				}
				if(!(siz.x & 3))
					sp++;
				if((siz.x & 3) <= 2)
					sp++;
			}
			break;

		case TYPE_C4_RLE:
			while(!eoi)
			{
				uint16 code = get16(sp);
				switch(code)
				{
					case 0x0000: x = 0; y += dy; break;
					case 0x0100: eoi = true; break;
					case 0x0200: x += int8(get8(sp)) * dx; y += int8(get8(sp)) * dy; break;
					default:
						if(code & 255)
						{
							// encoded run
							int count = code & 255;
							if(x + count > siz.x)
							{
								eoi = true;
								break;
							}
							uint32 c0 = palette[code >> 12];
							uint32 c1 = palette[(code >> 8) & 15];
							for(int i = 0; i < count / 2; i++)
							{
								pix.put8un(intxy(x++,y),c0);
								pix.put8un(intxy(x++,y),c1);
							}
							if(count & 1)
								pix.put8un(intxy(x++,y),c0);
						}
						else
						{
							// unencoded run
							int count = code >> 8;
							if(x + count > siz.x)
							{
								eoi = true;
								break;
							}
							for(int i = 0; i < count / 4; i++)
							{
								uint16 c = get16(sp);
								pix.put8un(intxy(x++,y),palette[(c >> 4) & 15]);
								pix.put8un(intxy(x++,y),palette[c & 15]);
								pix.put8un(intxy(x++,y),palette[c >> 12]);
								pix.put8un(intxy(x++,y),palette[(c >> 8) & 15]);
							}
							if(count & 3)
							{
								uint16 c = get16(sp);
								if((count & 3) >= 1)
									pix.put8un(intxy(x++,y),palette[(c >> 4) & 15]);
								if((count & 3) >= 2)
									pix.put8un(intxy(x++,y),palette[c & 15]);
								if((count & 3) >= 3)
									pix.put8un(intxy(x++,y),palette[c >> 12]);
							}
						}
						break;
				}
			}
			break;

		case TYPE_C8:
			for(; y != endy; y += dy)
				for(int x = 0; x < siz.x; x++)
					pix.put8un(intxy(x,y),palette[get8(sp)]);
			break;

		case TYPE_C8_RLE:
			while(!eoi)
			{
				uint16 code = get16(sp);
				switch(code)
				{
					case 0x0000: x = 0; y += dy; break;
					case 0x0100: eoi = true; break;
					case 0x0200: x += int8(get8(sp)) * dx; y += int8(get8(sp)) * dy; break;
					default:
						if(code & 255)
						{
							// encoded run
							int count = code & 255;
							if(x + count > siz.x)
							{
								eoi = true;
								break;
							}
							uint32 c = palette[code >> 8];
							for(int i = 0; i < count; i++)
								pix.put8un(intxy(x++,y),c);
						}
						else
						{
							// unencoded run
							int count = code >> 8;
							if(x + count > siz.x)
							{
								eoi = true;
								break;
							}
							for(int i = 0; i < count; i++)
							{
								uint16 c = get16(sp);
								pix.put8un(intxy(x++,y),palette[c & 255]);
								pix.put8un(intxy(x++,y),palette[c >> 8]);
							}
							if(count & 1)
							{
								uint16 c = get16(sp);
								pix.put8un(intxy(x++,y),palette[c & 255]);
							}
						}
						break;
				}
			}
			break;

		case TYPE_A1RGB5:
			for(; y != endy; y += dy)
				for(int x = 0; x < siz.x; x++)
				{
					uint16 d = get16(sp);
					uint8 r = (d >> 10) & 31;
					uint8 g = (d >> 5) & 31;
					uint8 b = d & 31;
					uint8 a = (d & 0x8000)?255:0;
					pix.put8un(intxy(x,y),(r << 3) | (r >> 2),(g << 3) | (g >> 2),(b << 3) | (b >> 2),a);
				}
			break;

		case TYPE_B16:
			for(; y != endy; y += dy)
				for(int x = 0; x < siz.x; x++)
				{
					uint16 c = get16(sp);
					uint r = red.get(c,0);
					uint g = green.get(c,0);
					uint b = blue.get(c,0);
					uint a = alpha.get(c,255);
					pix.put8un(intxy(x,y),r,g,b,a);
				}
			break;

		case TYPE_RGB8:
			for(; y != endy; y += dy)
				for(int x = 0; x < siz.x; x++)
				{
					uint8 b = get8(sp);
					uint8 g = get8(sp);
					uint8 r = get8(sp);
					pix.put8un(intxy(x,y),r,g,b,255);
				}
			break;

		case TYPE_ARGB8:
			for(; y != endy; y += dy)
				for(int x = 0; x < siz.x; x++)
					pix.put8un(intxy(x,y),get32(sp));
			break;

		case TYPE_B32:
			for(int y = 0; y < siz.y; y++)
				for(int x = 0; x < siz.x; x++)
				{
					uint16 c = get32(sp);
					uint r = red.get(c,0);
					uint g = green.get(c,0);
					uint b = blue.get(c,0);
					uint a = alpha.get(c,255);
					pix.put8un(intxy(x,y),r,g,b,a);
				}
			break;
	}
}

buffer imex PackBMP(const Image::pixels& pix,int bmpf,bool rle)
{
	intxy siz = pix.size();

	int headersize = 108;
	int stride = 0;
	int palettesize = 0;
	int bpp = 0;
	int compression = 0;
	int colors = 0;
	uint32 redmask = 0;
	uint32 greenmask = 0;
	uint32 bluemask = 0;
	uint32 alphamask = 0;
	switch(bmpf)
	{
		case BMP_L1:
		case BMP_C1:
			stride = (((siz.x + 7) / 8) + 3) & ~3;
			palettesize = 8;
			bpp = 1;
			colors = 2;
			break;

		case BMP_L4:
		case BMP_C4:
			stride = (((siz.x + 1) / 2) + 3) & ~3;
			palettesize = 64;
			bpp = 4;
			colors = 16;
			break;

		case BMP_L8:
		case BMP_C8:
			stride = (siz.x + 3) & ~3;
			palettesize = 1024;
			bpp = 8;
			colors = 256;
			break;

		case BMP_RGB5X1:
		case BMP_RGB5A1:
		case BMP_R5G6B5:
			stride = (siz.x * 2 + 3) & ~3;
			palettesize = 0;
			bpp = 16;
			compression = 3;
			switch(bmpf)
			{
				case BMP_RGB5X1:
					redmask = 0x00007C00;
					greenmask = 0x000003E0;
					bluemask = 0x0000001F;
					alphamask = 0x00000000;
					break;

				case BMP_RGB5A1:
					redmask = 0x00007C00;
					greenmask = 0x000003E0;
					bluemask = 0x0000001F;
					alphamask = 0x00008000;
					break;

				case BMP_R5G6B5:
					redmask = 0x0000F800;
					greenmask = 0x000007E0;
					bluemask = 0x0000001F;
					alphamask = 0x00000000;
					break;
			}
			break;

		case BMP_RGB8:
			stride = (siz.x * 3 + 3) & ~3;
			palettesize = 0;
			bpp = 24;
			break;

		case BMP_RGBX8:
		case BMP_RGBA8:
			stride = siz.x * 4;
			palettesize = 0;
			bpp = 32;
			compression = 3;
			switch(bmpf)
			{
				case BMP_RGBX8:
					redmask = 0x00FF0000;
					greenmask = 0x0000FF00;
					bluemask = 0x000000FF;
					alphamask = 0x00000000;
					break;

				case BMP_RGBA8:
					redmask = 0x00FF0000;
					greenmask = 0x0000FF00;
					bluemask = 0x000000FF;
					alphamask = 0xFF000000;
					break;
			}
			break;
	}

	int imagesize = stride * siz.y;
	int offset = 14 + headersize + palettesize;
	int filesize = offset + imagesize;

	buffer buf(filesize);
	uint8* ptr = buf.data();

	// write header header
	put16b(ptr,0x424D);   // 'BM'
	put32(ptr,filesize);  // total filesize
	put32(ptr,0);         // dummy
	put32(ptr,offset);    // offset to pixel data

	// write header
	put32(ptr,headersize);   // size of the header (version of the BMP)
	put32(ptr,siz.x);        // width
	put32(ptr,-siz.y);       // height (stored as right-side-up)
	put16(ptr,1);            // planes
	put16(ptr,bpp);          // bits per pixel
	put32(ptr,compression);  // compression type (one of COMP_)
	put32(ptr,imagesize);    // number of image data bytes
	put32(ptr,1);            // X pixels per meter
	put32(ptr,1);            // Y pixels per meter
	put32(ptr,colors);       // number of colors in color table
	put32(ptr,colors);       // number of important colors
	put32(ptr,redmask);      // red mask
	put32(ptr,greenmask);    // green mask
	put32(ptr,bluemask);     // blue mask
	put32(ptr,alphamask);    // alpha mask
	put32(ptr,0x57696E20);   // "Win " color space
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);   // gamma correction
	put32(ptr,0x00000000);
	put32(ptr,0x00000000);

	// write palette
	uint32 palette[256];
	uint8* lut = new uint8[32768];
	switch(bmpf)
	{
		case BMP_L1:
			put32(ptr,0xFF000000);
			put32(ptr,0xFFFFFFFF);
			break;

		case BMP_C1:
			CalcBestPalette(pix,palette,2,lut);
			put32(ptr,palette[0]);
			put32(ptr,palette[1]);
			break;

		case BMP_L4:
			for(uint i = 0; i < 16; i++)
			{
				uint l = (i * 255) / 15;
				put32(ptr,0xFF000000 | (l << 16) | (l << 8) | l);
			}
			break;

		case BMP_C4:
			CalcBestPalette(pix,palette,16,lut);
			for(uint i = 0; i < 16; i++)
				put32(ptr,palette[i]);
			break;

		case BMP_L8:
			for(uint i = 0; i < 256; i++)
				put32(ptr,0xFF000000 | (i << 16) | (i << 8) | i);
			break;

		case BMP_C8:
			CalcBestPalette(pix,palette,256,lut);
			for(uint i = 0; i < 256; i++)
				put32(ptr,palette[i]);
			break;
	}

	// encode pixels
	switch(bmpf)
	{
		case BMP_L1:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				int x;
				for(x = 0; x < siz.x / 8; x++)
				{
					uint8 d = 0;
					for(int i = 0; i < 8; i++)
					{
						uint8 r,g,b,a;
						pix.get8un(r,g,b,a,intxy(x * 8 + i,y));
						int l = ((r * 299) + (g * 587) + (b * 114)) / 1000;
						if(l & 0x80)
							d |= (0x80 >> i);
					}
					put8(ptr,d);
					count++;
				}
				if(siz.x & 7)
				{
					uint8 d = 0;
					for(int i = 0; i < (siz.x & 7); i++)
					{
						uint8 r,g,b,a;
						pix.get8un(r,g,b,a,intxy(x * 8 + i,y));
						int l = ((r * 299) + (g * 587) + (b * 114)) / 1000;
						if(l & 0x80)
							d |= (0x80 >> i);
					}
					put8(ptr,d);
					count++;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_C1:
			for(int y = 0; y < siz.y; y++)
			{
				int x;
				int count = 0;
				for(x = 0; x < siz.x / 8; x++)
				{
					uint8 d = 0;
					for(int i = 0; i < 8; i++)
					{
						uint8 r,g,b,a;
						pix.get8un(r,g,b,a,intxy(x * 8 + i,y));
						if(FindClosestColor(r,g,b,lut))
							d |= (0x80 >> i);
					}
					put8(ptr,d);
					count++;
				}
				if(siz.x & 7)
				{
					uint8 d = 0;
					for(int i = 0; i < (siz.x & 7); i++)
					{
						uint8 r,g,b,a;
						pix.get8un(r,g,b,a,intxy(x * 8 + i,y));
						if(FindClosestColor(r,g,b,lut))
							d |= (0x80 >> i);
					}
					put8(ptr,d);
					count++;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_L4:
			for(int y = 0; y < siz.y; y++)
			{
				int x;
				int count = 0;
				for(x = 0; x < siz.x / 2; x++)
				{
					uint8 d = 0;
					for(int i = 0; i < 2; i++)
					{
						uint8 r,g,b,a;
						pix.get8un(r,g,b,a,intxy(x * 2 + i,y));
						int l = ((r * 299) + (g * 587) + (b * 114)) / 1000;
						d |= (l & 0xF0) >> (4 * i);
					}
					put8(ptr,d);
					count++;
				}
				if(siz.x & 1)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x * 2,y));
					int l = ((r * 299) + (g * 587) + (b * 114)) / 1000;
					put8(ptr,l & 0xF0);
					count++;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_C4:
			for(int y = 0; y < siz.y; y++)
			{
				int x;
				int count = 0;
				for(x = 0; x < siz.x / 2; x++)
				{
					uint8 d = 0;
					for(int i = 0; i < 2; i++)
					{
						uint8 r,g,b,a;
						pix.get8un(r,g,b,a,intxy(x * 2 + i,y));
						int n = FindClosestColor(r,g,b,lut);
						d |= (n << 4) >> (4 * i);
					}
					put8(ptr,d);
					count++;
				}
				if(siz.x & 1)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x * 2,y));
					int n = FindClosestColor(r,g,b,lut);
					put8(ptr,n << 4);
					count++;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_L8:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				for(int x = 0; x < siz.x; x++)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x,y));
					put8(ptr,((r * 299) + (g * 587) + (b * 114)) / 1000);
					count++;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_C8:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				for(int x = 0; x < siz.x; x++)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x,y));
					int n = FindClosestColor(r,g,b,lut);
					put8(ptr,n);
					count++;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_RGB5X1:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				for(int x = 0; x < siz.x; x++)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x,y));
					r >>= 3;
					g >>= 3;
					b >>= 3;
					put16(ptr,(r << 10) | (g << 5) | b);
					count += 2;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_RGB5A1:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				for(int x = 0; x < siz.x; x++)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x,y));
					r >>= 3;
					g >>= 3;
					b >>= 3;
					put16(ptr,((a & 128)?0x8000:0) | (r << 10) | (g << 5) | b);
					count += 2;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_R5G6B5:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				for(int x = 0; x < siz.x; x++)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x,y));
					r >>= 3;
					g >>= 2;
					b >>= 3;
					put16(ptr,(r << 11) | (g << 5) | b);
					count += 2;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_RGB8:
			for(int y = 0; y < siz.y; y++)
			{
				int count = 0;
				for(int x = 0; x < siz.x; x++)
				{
					uint8 r,g,b,a;
					pix.get8un(r,g,b,a,intxy(x,y));
					put8(ptr,b);
					put8(ptr,g);
					put8(ptr,r);
					count += 3;
				}
				while(count & 3)
				{
					put8(ptr,0);
					count++;
				}
			}
			break;

		case BMP_RGBX8:
			for(int y = 0; y < siz.y; y++)
				for(int x = 0; x < siz.x; x++)
					put32(ptr,pix.get8un(intxy(x,y)) & 0x00FFFFFF);
			break;

		case BMP_RGBA8:
			for(int y = 0; y < siz.y; y++)
				for(int x = 0; x < siz.x; x++)
					put32(ptr,pix.get8un(intxy(x,y)));
			break;
	}

	delete[] lut;

	return buf;
}


}
*/