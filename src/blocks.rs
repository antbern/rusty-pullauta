use image::{Rgb, RgbImage, Rgba, RgbaImage};
use imageproc::drawing::draw_filled_rect_mut;
use imageproc::filter::median_filter;
use imageproc::rect::Rect;
use rustc_hash::FxHashMap as HashMap;
use std::error::Error;
use std::path::Path;

use crate::util::read_lines;

pub fn blocks(thread: &String) -> Result<(), Box<dyn Error>> {
    println!("Identifying blocks...");
    let tmpfolder = format!("temp{}", thread);
    let path = format!("{}/xyz2.xyz", tmpfolder);
    let xyz_file_in = Path::new(&path);
    let mut size: f64 = f64::NAN;
    let mut xstartxyz: f64 = f64::NAN;
    let mut ystartxyz: f64 = f64::NAN;
    let mut xmax: u64 = u64::MIN;
    let mut ymax: u64 = u64::MIN;
    if let Ok(lines) = read_lines(xyz_file_in) {
        for (i, line) in lines.enumerate() {
            let ip = line.unwrap_or(String::new());
            let mut parts = ip.split(' ');
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();

            if i == 0 {
                xstartxyz = x;
                ystartxyz = y;
            } else if i == 1 {
                size = y - ystartxyz;
            } else {
                break;
            }
        }
    }
    let mut xyz: HashMap<(u64, u64), f64> = HashMap::default();

    if let Ok(lines) = read_lines(xyz_file_in) {
        for line in lines {
            let ip = line.unwrap_or(String::new());
            let mut parts = ip.split(' ');
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let h: f64 = parts.next().unwrap().parse::<f64>().unwrap();

            let xx = ((x - xstartxyz) / size).floor() as u64;
            let yy = ((y - ystartxyz) / size).floor() as u64;
            xyz.insert((xx, yy), h);

            if xmax < xx {
                xmax = xx;
            }
            if ymax < yy {
                ymax = yy;
            }
        }
    }
    let mut img = RgbImage::from_pixel(xmax as u32 * 2, ymax as u32 * 2, Rgb([255, 255, 255]));
    let mut img2 = RgbaImage::from_pixel(xmax as u32 * 2, ymax as u32 * 2, Rgba([0, 0, 0, 0]));

    let black = Rgb([0, 0, 0]);
    let white = Rgba([255, 255, 255, 255]);

    let path = format!("{}/xyztemp.xyz", tmpfolder);
    let xyz_file_in = Path::new(&path);
    if let Ok(lines) = read_lines(xyz_file_in) {
        for line in lines {
            let ip = line.unwrap_or(String::new());
            let mut parts = ip.split(' ');
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let h: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let r3 = parts.next().unwrap();
            let r4 = parts.next().unwrap();
            let r5 = parts.next().unwrap();

            let xx = ((x - xstartxyz) / size).floor() as u64;
            let yy = ((y - ystartxyz) / size).floor() as u64;
            if r3 != "2"
                && r3 != "9"
                && r4 == "1"
                && r5 == "1"
                && h - *xyz.get(&(xx, yy)).unwrap_or(&0.0) > 2.0
            {
                draw_filled_rect_mut(
                    &mut img,
                    Rect::at(
                        (x - xstartxyz - 1.0) as i32,
                        (ystartxyz + 2.0 * ymax as f64 - y - 1.0) as i32,
                    )
                    .of_size(3, 3),
                    black,
                );
            } else {
                draw_filled_rect_mut(
                    &mut img2,
                    Rect::at(
                        (x - xstartxyz - 1.0) as i32,
                        (ystartxyz + 2.0 * ymax as f64 - y - 1.0) as i32,
                    )
                    .of_size(3, 3),
                    white,
                );
            }
        }
    }
    let filter_size = 2;
    img.save(Path::new(&format!("{}/blocks.png", tmpfolder)))
        .expect("error saving png");
    img2.save(Path::new(&format!("{}/blocks2.png", tmpfolder)))
        .expect("error saving png");
    let mut img =
        image::open(Path::new(&format!("{}/blocks.png", tmpfolder))).expect("Opening image failed");
    let img2 = image::open(Path::new(&format!("{}/blocks2.png", tmpfolder)))
        .expect("Opening image failed");

    image::imageops::overlay(&mut img, &img2, 0, 0);

    img = image::DynamicImage::ImageRgb8(median_filter(&img.to_rgb8(), filter_size, filter_size));

    img.save(Path::new(&format!("{}/blocks.png", tmpfolder)))
        .expect("error saving png");
    println!("Done");
    Ok(())
}
