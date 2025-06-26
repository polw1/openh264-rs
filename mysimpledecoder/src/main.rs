mod decoder;

use decoder::{Decoder, Error};
use std::fs::File;
use std::io::{Read, Write};

const NAL_MIN_0_COUNT: usize = 2;

fn nth_nal_index(stream: &[u8], nth: usize) -> Option<usize> {
    let mut count_0 = 0;
    let mut n = 0;
    for (i, byte) in stream.iter().enumerate() {
        match byte {
            0 => count_0 += 1,
            1 if count_0 >= NAL_MIN_0_COUNT => {
                if n == nth {
                    return Some(i - NAL_MIN_0_COUNT);
                }
                count_0 = 0;
                n += 1;
            }
            _ => count_0 = 0,
        }
    }
    None
}

fn nal_units(mut stream: &[u8]) -> impl Iterator<Item = &[u8]> {
    std::iter::from_fn(move || {
        let first = nth_nal_index(stream, 0);
        let next = nth_nal_index(stream, 1);
        match (first, next) {
            (Some(f), Some(n)) => {
                let val = &stream[f..n];
                stream = &stream[n..];
                Some(val)
            }
            (Some(f), None) => {
                let val = &stream[f..];
                stream = &stream[f + NAL_MIN_0_COUNT..];
                Some(val)
            }
            _ => None,
        }
    })
}

fn write_ppm(path: &str, rgb: &[u8], width: usize, height: usize) -> Result<(), std::io::Error> {
    let mut file = File::create(path)?;
    writeln!(file, "P6\n{} {}\n255", width, height)?;
    file.write_all(rgb)?;
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut args = std::env::args();
    let bin = args.next().unwrap_or_else(|| "mysimpledecoder".into());
    let input = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: {bin} <h264-file> [output.ppm]");
            std::process::exit(1);
        }
    };
    let output = args.next().unwrap_or_else(|| "frame.ppm".into());

    let mut data = Vec::new();
    File::open(&input)?.read_to_end(&mut data)?;

    let mut decoder = Decoder::new()?;

    let mut rgb = Vec::new();
    let mut width = 0usize;
    let mut height = 0usize;

    for packet in nal_units(&data) {
        match decoder.decode(packet)? {
            Some(image) => {
                let dim = image.dimensions();
                width = dim.0;
                height = dim.1;
                rgb.resize(width * height * 3, 0);
                image.write_rgb8(&mut rgb);
                break;
            }
            None => continue,
        }
    }

    if width == 0 || height == 0 {
        eprintln!("No frame decoded");
        return Ok(());
    }

    write_ppm(&output, &rgb, width, height)?;
    println!("Wrote first frame to {output}");

    Ok(())
}
