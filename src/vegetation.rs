use image::{GrayImage, Luma, Rgb, RgbImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_filled_rect_mut, draw_line_segment_mut};
use imageproc::filter::median_filter;
use imageproc::rect::Rect;
use log::info;
use rustc_hash::FxHashMap as HashMap;
use std::error::Error;
use std::f32::consts::SQRT_2;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use crate::config::{Config, Zone};
use crate::util::{read_lines, read_lines_no_alloc};

pub fn makevege(config: &Config, tmpfolder: &Path) -> Result<(), Box<dyn Error>> {
    info!("Generating vegetation...");

    let path = tmpfolder.join("xyz2.xyz");
    let xyz_file_in = Path::new(&path);

    let mut xstart: f64 = 0.0;
    let mut ystart: f64 = 0.0;
    let mut size: f64 = 0.0;

    if let Ok(lines) = read_lines(xyz_file_in) {
        for (i, line) in lines.enumerate() {
            let ip = line.unwrap_or(String::new());
            let mut parts = ip.split(' ');
            let x = parts.next().unwrap().parse::<f64>().unwrap();
            let y = parts.next().unwrap().parse::<f64>().unwrap();

            if i == 0 {
                xstart = x;
                ystart = y;
            } else if i == 1 {
                size = y - ystart;
            } else {
                break;
            }
        }
    }

    let block = config.greendetectsize;

    let mut xyz: HashMap<(u64, u64), f64> = HashMap::default();
    let mut top: HashMap<(u64, u64), f64> = HashMap::default();

    read_lines_no_alloc(xyz_file_in, |line| {
        let mut parts = line.trim().split(' ');

        let x = parts.next().unwrap().parse::<f64>().unwrap();
        let y = parts.next().unwrap().parse::<f64>().unwrap();
        let h = parts.next().unwrap().parse::<f64>().unwrap();

        let xx = ((x - xstart) / size).floor() as u64;
        let yy = ((y - ystart) / size).floor() as u64;
        xyz.insert((xx, yy), h);
        let xxx = ((x - xstart) / block).floor() as u64;
        let yyy = ((y - ystart) / block).floor() as u64;
        if top.contains_key(&(xxx, yyy)) && h > *top.get(&(xxx, yyy)).unwrap() {
            top.insert((xxx, yyy), h);
        }
    })
    .expect("Can not read file");

    let thresholds = &config.thresholds;

    let &Config {
        vege_bitmode,
        yellowheight,
        yellowthreshold,
        greenground,
        pointvolumefactor,
        pointvolumeexponent,
        greenhigh,
        topweight,
        greentone,
        vegezoffset: zoffset,
        uglimit,
        uglimit2,
        addition,
        firstandlastreturnasground,
        firstandlastfactor,
        lastfactor,
        yellowfirstlast,
        vegethin,
        ..
    } = config;
    let greenshades = &config.greenshades;

    let path = tmpfolder.join("xyztemp.xyz");
    let xyz_file_in = Path::new(&path);

    let xmin = xstart;
    let ymin = ystart;
    let mut xmax: f64 = f64::MIN;
    let mut ymax: f64 = f64::MIN;

    let mut hits: HashMap<(u64, u64), u64> = HashMap::default();
    let mut yhit: HashMap<(u64, u64), u64> = HashMap::default();
    let mut noyhit: HashMap<(u64, u64), u64> = HashMap::default();

    let mut i = 0;
    read_lines_no_alloc(xyz_file_in, |line| {
        if vegethin == 0 || ((i + 1) as u32) % vegethin == 0 {
            let mut parts = line.trim().split(' ');
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let h: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let r3 = parts.next().unwrap();
            let r4 = parts.next().unwrap();
            let r5 = parts.next().unwrap();

            if xmax < x {
                xmax = x;
            }
            if ymax < y {
                ymax = y;
            }
            if x > xmin && y > ymin {
                let xx = ((x - xmin) / block).floor() as u64;
                let yy = ((y - ymin) / block).floor() as u64;
                if h > *top.get(&(xx, yy)).unwrap_or(&0.0) {
                    top.insert((xx, yy), h);
                }
                let xx = ((x - xmin) / 3.0).floor() as u64;
                let yy = ((y - ymin) / 3.0).floor() as u64;
                *hits.entry((xx, yy)).or_insert(0) += 1;

                if r3 == "2"
                    || h < yellowheight
                        + *xyz
                            .get(&(
                                ((x - xmin) / size).floor() as u64,
                                ((y - ymin) / size).floor() as u64,
                            ))
                            .unwrap_or(&0.0)
                {
                    *yhit.entry((xx, yy)).or_insert(0) += 1;
                } else if r4 == "1" && r5 == "1" {
                    *noyhit.entry((xx, yy)).or_insert(0) += yellowfirstlast;
                } else {
                    *noyhit.entry((xx, yy)).or_insert(0) += 1;
                }
            }
        }

        i += 1;
    })
    .expect("Can not read file");

    let mut firsthit: HashMap<(u64, u64), u64> = HashMap::default();
    let mut ugg: HashMap<(u64, u64), f64> = HashMap::default();
    let mut ug: HashMap<(u64, u64), u64> = HashMap::default();
    let mut ghit: HashMap<(u64, u64), u64> = HashMap::default();
    let mut greenhit: HashMap<(u64, u64), f64> = HashMap::default();
    let mut highit: HashMap<(u64, u64), u64> = HashMap::default();
    let step: f32 = 6.0;

    let mut i = 0;
    read_lines_no_alloc(xyz_file_in, |line| {
        if vegethin == 0 || ((i + 1) as u32) % vegethin == 0 {
            let mut parts = line.trim().split(' ');

            // parse the parts of the line
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let h: f64 = parts.next().unwrap().parse::<f64>().unwrap() - zoffset;
            let r3 = parts.next().unwrap();
            let r4 = parts.next().unwrap();
            let r5 = parts.next().unwrap();

            if x > xmin && y > ymin {
                if r5 == "1" {
                    let xx = ((x - xmin) / block + 0.5).floor() as u64;
                    let yy = ((y - ymin) / block + 0.5).floor() as u64;
                    *firsthit.entry((xx, yy)).or_insert(0) += 1;
                }

                let xx = ((x - xmin) / size).floor() as u64;
                let yy = ((y - ymin) / size).floor() as u64;
                let a = *xyz.get(&(xx, yy)).unwrap_or(&0.0);
                let b = *xyz.get(&(xx + 1, yy)).unwrap_or(&0.0);
                let c = *xyz.get(&(xx, yy + 1)).unwrap_or(&0.0);
                let d = *xyz.get(&(xx + 1, yy + 1)).unwrap_or(&0.0);

                let distx = (x - xmin) / size - xx as f64;
                let disty = (y - ymin) / size - yy as f64;

                let ab = a * (1.0 - distx) + b * distx;
                let cd = c * (1.0 - distx) + d * distx;
                let thelele = ab * (1.0 - disty) + cd * disty;
                let xx = ((x - xmin) / block / (step as f64) + 0.5).floor() as u64;
                let yy = (((y - ymin) / block / (step as f64)).floor() + 0.5).floor() as u64;
                let hh = h - thelele;
                if hh <= 1.2 {
                    if r3 == "2" {
                        *ugg.entry((xx, yy)).or_insert(0.0) += 1.0;
                    } else if hh > 0.25 {
                        *ug.entry((xx, yy)).or_insert(0) += 1;
                    } else {
                        *ugg.entry((xx, yy)).or_insert(0.0) += 1.0;
                    }
                } else {
                    *ugg.entry((xx, yy)).or_insert(0.0) += 0.05;
                }

                let xx = ((x - xmin) / block + 0.5).floor() as u64;
                let yy = ((y - ymin) / block + 0.5).floor() as u64;
                let yyy = ((y - ymin) / block).floor() as u64; // necessary due to bug in perl version
                if r3 == "2" || greenground >= hh {
                    if r4 == "1" && r5 == "1" {
                        *ghit.entry((xx, yyy)).or_insert(0) += firstandlastreturnasground;
                    } else {
                        *ghit.entry((xx, yyy)).or_insert(0) += 1;
                    }
                } else {
                    let mut last = 1.0;
                    if r4 == r5 {
                        last = lastfactor;
                        if hh < 5.0 {
                            last = firstandlastfactor;
                        }
                    }
                    for &Zone {
                        low,
                        high,
                        roof,
                        factor,
                    } in config.zones.iter()
                    {
                        if hh >= low
                            && hh < high
                            && *top.get(&(xx, yy)).unwrap_or(&0.0) - thelele < roof
                        {
                            let offset = factor * last;
                            *greenhit.entry((xx, yy)).or_insert(0.0) += offset;
                            break;
                        }
                    }

                    if greenhigh < hh {
                        *highit.entry((xx, yy)).or_insert(0) += 1;
                    }
                }
            }
        }

        i += 1;
    })
    .expect("Can not read file");

    let w = (xmax - xmin).floor() / block;
    let h = (ymax - ymin).floor() / block;
    let wy = (xmax - xmin).floor() / 3.0;
    let hy = (ymax - ymin).floor() / 3.0;

    let scalefactor = config.scalefactor;

    let mut imgug = RgbaImage::from_pixel(
        (w * block * 600.0 / 254.0 / scalefactor) as u32,
        (h * block * 600.0 / 254.0 / scalefactor) as u32,
        Rgba([255, 255, 255, 0]),
    );
    let mut img_ug_bit = GrayImage::from_pixel(
        (w * block * 600.0 / 254.0 / scalefactor) as u32,
        (h * block * 600.0 / 254.0 / scalefactor) as u32,
        Luma([0x00]),
    );
    let mut imggr1 =
        RgbImage::from_pixel((w * block) as u32, (h * block) as u32, Rgb([255, 255, 255]));
    let mut imggr1b =
        RgbImage::from_pixel((w * block) as u32, (h * block) as u32, Rgb([255, 255, 255]));
    let mut imgye2 = RgbaImage::from_pixel(
        (w * block) as u32,
        (h * block) as u32,
        Rgba([255, 255, 255, 0]),
    );
    let mut imgye2b = RgbaImage::from_pixel(
        (w * block) as u32,
        (h * block) as u32,
        Rgba([255, 255, 255, 0]),
    );
    let mut imgwater =
        RgbImage::from_pixel((w * block) as u32, (h * block) as u32, Rgb([255, 255, 255]));

    let mut greens = Vec::new();
    for i in 0..greenshades.len() {
        greens.push(Rgb([
            (greentone - greentone / (greenshades.len() - 1) as f64 * i as f64) as u8,
            (254.0 - (74.0 / (greenshades.len() - 1) as f64) * i as f64) as u8,
            (greentone - greentone / (greenshades.len() - 1) as f64 * i as f64) as u8,
        ]))
    }

    let mut aveg = 0;
    let mut avecount = 0;

    for x in 1..(h as usize) {
        for y in 1..(h as usize) {
            let xx = x as u64;
            let yy = y as u64;
            if *ghit.get(&(xx, yy)).unwrap_or(&0) > 1 {
                aveg += *firsthit.get(&(xx, yy)).unwrap_or(&0);
                avecount += 1;
            }
        }
    }
    let aveg = aveg as f64 / avecount as f64;
    let ye2 = Rgba([255, 219, 166, 255]);
    for x in 4..(wy as usize - 3) {
        for y in 4..(hy as usize - 3) {
            let mut ghit2 = 0;
            let mut highhit2 = 0;

            for i in x..x + 2 {
                for j in y..y + 2 {
                    ghit2 += *yhit.get(&(i as u64, j as u64)).unwrap_or(&0);
                    highhit2 += *noyhit.get(&(i as u64, j as u64)).unwrap_or(&0);
                }
            }
            if ghit2 as f64 / (highhit2 as f64 + ghit2 as f64 + 0.01) > yellowthreshold {
                draw_filled_rect_mut(
                    &mut imgye2,
                    Rect::at(x as i32 * 3 + 2, (hy as i32 - y as i32) * 3 - 3).of_size(3, 3),
                    ye2,
                );
            }
        }
    }

    for x in 2..w as usize {
        for y in 2..h as usize {
            let mut ghit2 = 0;
            let mut highit2 = 0;
            let roof = *top.get(&(x as u64, y as u64)).unwrap_or(&0.0)
                - *xyz
                    .get(&(
                        (x as f64 * block / size).floor() as u64,
                        (y as f64 * block / size).floor() as u64,
                    ))
                    .unwrap_or(&0.0);

            let greenhit2 = *greenhit.get(&(x as u64, y as u64)).unwrap_or(&0.0);
            let mut firsthit2 = *firsthit.get(&(x as u64, y as u64)).unwrap_or(&0);
            for i in (x - 2)..x + 3_usize {
                for j in (y - 2)..y + 3_usize {
                    if firsthit2 > *firsthit.get(&(i as u64, j as u64)).unwrap_or(&0) {
                        firsthit2 = *firsthit.get(&(i as u64, j as u64)).unwrap_or(&0);
                    }
                }
            }
            highit2 += *highit.get(&(x as u64, y as u64)).unwrap_or(&0);
            ghit2 += *ghit.get(&(x as u64, y as u64)).unwrap_or(&0);

            let mut greenlimit = 9999.0;
            for &(v0, v1, v2) in thresholds.iter() {
                if roof >= v0 && roof < v1 {
                    greenlimit = v2;
                    break;
                }
            }

            let mut greenshade = 0;

            let thevalue = greenhit2 / (ghit2 as f64 + greenhit2 + 1.0)
                * (1.0 - topweight
                    + topweight * highit2 as f64
                        / (ghit2 as f64 + greenhit2 + highit2 as f64 + 1.0))
                * (1.0 - pointvolumefactor * firsthit2 as f64 / (aveg + 0.00001))
                    .powf(pointvolumeexponent);
            if thevalue > 0.0 {
                for (i, &shade) in greenshades.iter().enumerate() {
                    if thevalue > greenlimit * shade {
                        greenshade = i + 1;
                    }
                }
                if greenshade > 0 {
                    draw_filled_rect_mut(
                        &mut imggr1,
                        Rect::at(
                            ((x as f64 + 0.5) * block) as i32 - addition,
                            (((h - y as f64) - 0.5) * block) as i32 - addition,
                        )
                        .of_size(
                            (block as i32 + addition) as u32,
                            (block as i32 + addition) as u32,
                        ),
                        *greens.get(greenshade - 1).unwrap(),
                    );
                }
            }
        }
    }

    let proceed_yellows: bool = config.proceed_yellows;
    let med: u32 = config.med;
    let med2 = config.med2;

    if med > 0 {
        imggr1b = median_filter(&imggr1, med / 2, med / 2);
        if proceed_yellows {
            imgye2b = median_filter(&imgye2, med / 2, med / 2);
        }
    }
    if med2 > 0 {
        imggr1 = median_filter(&imggr1b, med2 / 2, med2 / 2);
        if proceed_yellows {
            imgye2 = median_filter(&imgye2, med / 2, med / 2);
        }
    } else {
        imggr1 = imggr1b;
        if proceed_yellows {
            imgye2 = imgye2b;
        }
    }

    imgye2
        .save(tmpfolder.join("yellow.png"))
        .expect("could not save output png");
    imggr1
        .save(tmpfolder.join("greens.png"))
        .expect("could not save output png");

    let mut img = image::open(tmpfolder.join("greens.png")).expect("Opening image failed");
    let img2 = image::open(tmpfolder.join("yellow.png")).expect("Opening image failed");
    image::imageops::overlay(&mut img, &img2, 0, 0);
    img.save(tmpfolder.join("vegetation.png"))
        .expect("could not save output png");

    if vege_bitmode {
        let g_img = image::open(tmpfolder.join("greens.png")).expect("Opening image failed");
        let mut g_img = g_img.to_rgb8();
        for pixel in g_img.pixels_mut() {
            let mut found = false;
            for (idx, color) in greens.iter().enumerate() {
                let c = idx as u8 + 2;
                if pixel[0] == color[0] && pixel[1] == color[1] && pixel[2] == color[2] {
                    *pixel = Rgb([c, c, c]);
                    found = true;
                }
            }
            if !found {
                *pixel = Rgb([0, 0, 0]);
            }
        }
        g_img
            .save(tmpfolder.join("greens_bit.png"))
            .expect("could not save output png");
        let g_img = image::open(tmpfolder.join("greens_bit.png")).expect("Opening image failed");
        let g_img = g_img.to_luma8();
        g_img
            .save(tmpfolder.join("greens_bit.png"))
            .expect("could not save output png");

        let y_img = image::open(tmpfolder.join("yellow.png")).expect("Opening image failed");
        let mut y_img = y_img.to_rgba8();
        for pixel in y_img.pixels_mut() {
            if pixel[0] == ye2[0] && pixel[1] == ye2[1] && pixel[2] == ye2[2] && pixel[3] == ye2[3]
            {
                *pixel = Rgba([1, 1, 1, 255]);
            } else {
                *pixel = Rgba([0, 0, 0, 0]);
            }
        }
        y_img
            .save(tmpfolder.join("yellow_bit.png"))
            .expect("could not save output png");
        let y_img = image::open(tmpfolder.join("yellow_bit.png")).expect("Opening image failed");
        let y_img = y_img.to_luma_alpha8();
        y_img
            .save(tmpfolder.join("yellow_bit.png"))
            .expect("could not save output png");

        let mut img_bit =
            image::open(tmpfolder.join("greens_bit.png")).expect("Opening image failed");
        let img_bit2 = image::open(tmpfolder.join("yellow_bit.png")).expect("Opening image failed");
        image::imageops::overlay(&mut img_bit, &img_bit2, 0, 0);
        img_bit
            .save(tmpfolder.join("vegetation_bit.png"))
            .expect("could not save output png");
    }

    let black = Rgb([0, 0, 0]);
    let blue = Rgb([29, 190, 255]);
    let buildings = config.buildings;
    let water = config.water;
    if buildings > 0 || water > 0 {
        read_lines_no_alloc(xyz_file_in, |line| {
            let mut parts = line.split(' ');
            let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
            parts.next();
            let c: u64 = parts.next().unwrap().parse::<u64>().unwrap();

            if c == buildings {
                draw_filled_rect_mut(
                    &mut imgwater,
                    Rect::at((x - xmin) as i32 - 1, (ymax - y) as i32 - 1).of_size(3, 3),
                    black,
                );
            }
            if c == water {
                draw_filled_rect_mut(
                    &mut imgwater,
                    Rect::at((x - xmin) as i32 - 1, (ymax - y) as i32 - 1).of_size(3, 3),
                    blue,
                );
            }
        })
        .expect("Can not read file");
    }

    let xyz_file_in = tmpfolder.join("xyz2.xyz");
    read_lines_no_alloc(xyz_file_in, |line| {
        let mut parts = line.split(' ');
        let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let hh: f64 = parts.next().unwrap().parse::<f64>().unwrap();

        if hh < config.waterele {
            draw_filled_rect_mut(
                &mut imgwater,
                Rect::at((x - xmin) as i32 - 1, (ymax - y) as i32 - 1).of_size(3, 3),
                blue,
            );
        }
    })
    .expect("Can not read file");

    imgwater
        .save(tmpfolder.join("blueblack.png"))
        .expect("could not save output png");

    let underg = Rgba([64, 121, 0, 255]);
    let tmpfactor = (600.0 / 254.0 / scalefactor) as f32;

    let bf32 = block as f32;
    let hf32 = h as f32;
    let ww = w as f32 * bf32;
    let hh = hf32 * bf32;
    let mut x = 0.0_f32;

    loop {
        if x >= ww {
            break;
        }
        let mut y = 0.0_f32;
        loop {
            if y >= hh {
                break;
            }
            let xx = ((x / bf32 / step).floor()) as u64;
            let yy = ((y / bf32 / step).floor()) as u64;
            let value = *ug.get(&(xx, yy)).unwrap_or(&0) as f64
                / (*ug.get(&(xx, yy)).unwrap_or(&0) as f64
                    + { *ugg.get(&(xx, yy)).unwrap_or(&0.0) }
                    + 0.01);
            if value > uglimit {
                draw_line_segment_mut(
                    &mut imgug,
                    (
                        tmpfactor * (x + bf32 * 3.0),
                        tmpfactor * (hf32 * bf32 - y - bf32 * 3.0),
                    ),
                    (
                        tmpfactor * (x + bf32 * 3.0),
                        tmpfactor * (hf32 * bf32 - y + bf32 * 3.0),
                    ),
                    underg,
                );
                draw_line_segment_mut(
                    &mut imgug,
                    (
                        tmpfactor * (x + bf32 * 3.0) + 1.0,
                        tmpfactor * (hf32 * bf32 - y - bf32 * 3.0),
                    ),
                    (
                        tmpfactor * (x + bf32 * 3.0) + 1.0,
                        tmpfactor * (hf32 * bf32 - y + bf32 * 3.0),
                    ),
                    underg,
                );
                draw_line_segment_mut(
                    &mut imgug,
                    (
                        tmpfactor * (x - bf32 * 3.0),
                        tmpfactor * (hf32 * bf32 - y - bf32 * 3.0),
                    ),
                    (
                        tmpfactor * (x - bf32 * 3.0),
                        tmpfactor * (hf32 * bf32 - y + bf32 * 3.0),
                    ),
                    underg,
                );
                draw_line_segment_mut(
                    &mut imgug,
                    (
                        tmpfactor * (x - bf32 * 3.0) + 1.0,
                        tmpfactor * (hf32 * bf32 - y - bf32 * 3.0),
                    ),
                    (
                        tmpfactor * (x - bf32 * 3.0) + 1.0,
                        tmpfactor * (hf32 * bf32 - y + bf32 * 3.0),
                    ),
                    underg,
                );

                if vege_bitmode {
                    draw_filled_circle_mut(
                        &mut img_ug_bit,
                        (
                            (tmpfactor * (x)) as i32,
                            (tmpfactor * (hf32 * bf32 - y)) as i32,
                        ),
                        (bf32 * 9.0 * SQRT_2) as i32,
                        Luma([0x01]),
                    )
                }
            }
            if value > uglimit2 {
                draw_line_segment_mut(
                    &mut imgug,
                    (tmpfactor * x, tmpfactor * (hf32 * bf32 - y - bf32 * 3.0)),
                    (tmpfactor * x, tmpfactor * (hf32 * bf32 - y + bf32 * 3.0)),
                    underg,
                );
                draw_line_segment_mut(
                    &mut imgug,
                    (
                        tmpfactor * x + 1.0,
                        tmpfactor * (hf32 * bf32 - y - bf32 * 3.0),
                    ),
                    (
                        tmpfactor * x + 1.0,
                        tmpfactor * (hf32 * bf32 - y + bf32 * 3.0),
                    ),
                    underg,
                );

                if vege_bitmode {
                    draw_filled_circle_mut(
                        &mut img_ug_bit,
                        (
                            (tmpfactor * (x)) as i32,
                            (tmpfactor * (hf32 * bf32 - y)) as i32,
                        ),
                        (bf32 * 9.0 * SQRT_2) as i32,
                        Luma([0x02]),
                    )
                }
            }

            y += bf32 * step;
        }
        x += bf32 * step;
    }
    imgug
        .save(tmpfolder.join("undergrowth.png"))
        .expect("could not save output png");
    let img_ug_bit_b = median_filter(&img_ug_bit, (bf32 * step) as u32, (bf32 * step) as u32);
    img_ug_bit_b
        .save(tmpfolder.join("undergrowth_bit.png"))
        .expect("could not save output png");

    let ugpgw = File::create(tmpfolder.join("undergrowth.pgw")).expect("Unable to create file");
    let mut ugpgw = BufWriter::new(ugpgw);
    write!(
        &mut ugpgw,
        "{}\r\n0.0\r\n0.0\r\n{}\r\n{}\r\n{}\r\n",
        1.0 / tmpfactor,
        -1.0 / tmpfactor,
        xmin,
        ymax,
    )
    .expect("Cannot write pgw file");

    let vegepgw = File::create(tmpfolder.join("vegetation.pgw")).expect("Unable to create file");
    let mut vegepgw = BufWriter::new(vegepgw);
    write!(
        &mut vegepgw,
        "1.0\r\n0.0\r\n0.0\r\n-1.0\r\n{}\r\n{}\r\n",
        xmin, ymax
    )
    .expect("Cannot write pgw file");

    info!("Done");
    Ok(())
}
