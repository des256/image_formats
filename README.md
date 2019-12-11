# image_formats
MIT NIHS image format implementations, version 0.1.0

Desmond Germans, 2019

If you use this, mention my name somewhere, and also, do one good deed for your friends.

*WARNING: NIHS stands for Not Invented Here Syndrome. If this bothers you, please use the official image format crates (there are a few, for instance `image-0.23.0`) and associated dependencies.*

## format coverage

format | decode         | encode        | tested        | optimization
-------|----------------|---------------|---------------|-------------
BMP    | **yes**        | *in progress* | **yes**       |
PNG    | **yes**        | *soon*        | **yes**       | improved huffman for LZ77
JPEG   | *in progress*  | *soon*        | *in progress* |
GIF    | *later*        | *later*       |               |
TGA    | *later*        | *later*       |               |
PBM    | *later*        | *later*       |               |
TIFF   | *later*        | *later*       |               |
XBM    | *later*        | *later*       |               |
WEBP   | *later*        | *later*       |               |

## how to use

Each format has a `test`, `decode` and `encode` function, in their own namespace (`bmp`, `png`, `jpeg`, etc.).

### `Image`

A small struct that stores resolution (image.width, image.height) and pixel data (image.data). The pixels are stored as `u32`, in ARGB order.

### `fn test(bytes: &[u8]) -> Option<(usize,usize)>`

This tests if `bytes` are a valid image of that format. Returns `Some((width,height))` if valid. Returns `None` otherwise.

### `fn decode(bytes: &[u8]) -> Result<Image,String>`

Decodes `bytes` in that format. If succesful, returns `Ok(image)`, otherwise it returns `Err(message)`.

### `fn encode(image: &Image) -> Result<Vec<u8>,String>`

Encodes `image` in that format. If succesful, returns the encoded bytes as `Ok(Vec<u8>)`, otherwise it returns `Err(message)`.

### examples

Load a BMP file:

```
use image_formats::bmp;

fn load_as_bmp(name: &str) {
    let mut file = File::open(&name).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    match bmp::decode(&buffer) {
        Ok(image) => {
            println!("Loaded. The image is {}x{} pixels.",image.width,image.height);
        },
        Err(msg) => {
            println!("Error: {}",msg);
        }
    }
}
```

Save image as PNG:

```
use image_formats::png;

fn save_as_png(name: &str,image: &Image) {
    match png::encode(&image) {
        Ok(bytes) => {
            let mut file = File::create(&name).unwrap();
            file.write_all(&bytes).unwrap();
        },
        Err(msg) => {
            println!("Error: {}",msg);
        }
    }
}
```

Load any format:

```
use image_formats::*;

fn load(name: &str) -> Result<Image,String> {
    let mut file = File::open(&name).unwrap();
    let mut buffer = Vec::new();
    if let Some((width,height)) = png::test(&buffer) {
        return png::decode(&buffer);
    }
    if let Some((width,height)) = bmp::test(&buffer) {
        return bmp::decode(&buffer);
    }
    if let Some((width,height)) = jpeg::test(&buffer) {
        return jpeg::decode(&buffer);
    }

    // ...

    return Err("unknown format".to_string());
}
```

## But, but, my image doesn't work?!

email it to me at desmond@germansmedia.nl
