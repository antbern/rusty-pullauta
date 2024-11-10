use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};
use imageproc::{drawing::draw_filled_rect_mut, filter::median_filter, rect::Rect};
use log::info;
use rustc_hash::FxHashMap as HashMap;
use std::error::Error;

use crate::util::FileProvider;

pub fn blocks<P: FileProvider>(provider: &mut P) -> Result<(), Box<dyn Error>> {
    info!("Identifying blocks...");
    let xyz_file_in = "xyz2.xyz";
    let mut size: f64 = f64::NAN;
    let mut xstartxyz: f64 = f64::NAN;
    let mut ystartxyz: f64 = f64::NAN;
    let mut xmax: u64 = u64::MIN;
    let mut ymax: u64 = u64::MIN;

    let mut i = 0;

    provider
        .lines(xyz_file_in, |line| {
            let mut parts = line.split(' ');
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();

            if i == 0 {
                xstartxyz = x;
                ystartxyz = y;
            } else if i == 1 {
                size = y - ystartxyz;
            } else {
                return Some(());
            }
            i += 1;
            None
        })
        .expect("could not read input file");

    let mut xyz: HashMap<(u64, u64), f64> = HashMap::default();
    provider
        .lines(xyz_file_in, |line| {
            let mut parts = line.split(' ');
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
            None::<()>
        })
        .expect("could not read input file");

    let mut img = RgbImage::from_pixel(xmax as u32 * 2, ymax as u32 * 2, Rgb([255, 255, 255]));
    let mut img2 = RgbaImage::from_pixel(xmax as u32 * 2, ymax as u32 * 2, Rgba([0, 0, 0, 0]));

    let black = Rgb([0, 0, 0]);
    let white = Rgba([255, 255, 255, 255]);

    provider
        .lines("xyztemp.xyz", |line| {
            let mut parts = line.split(' ');
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
            None::<()>
        })
        .expect("could not read input file");

    img2.save(provider.path("blocks2.png"))
        .expect("error saving png");

    let mut img = DynamicImage::ImageRgb8(img);

    image::imageops::overlay(&mut img, &DynamicImage::ImageRgba8(img2), 0, 0);

    let filter_size = 2;
    img = image::DynamicImage::ImageRgb8(median_filter(&img.to_rgb8(), filter_size, filter_size));

    img.save(provider.path("blocks.png"))
        .expect("error saving png");
    info!("Done");
    Ok(())
}
