use log::info;
use rustc_hash::FxHashMap as HashMap;
use std::error::Error;
use std::path::Path;

use crate::config::Config;
use crate::geometry::{BinaryDxf, Bounds, Classification, Point2, Polylines};
use crate::io::fs::FileSystem;
use crate::io::heightmap::HeightMap;
use crate::io::xyz::XyzInternalReader;
use crate::vec2d::Vec2D;

/// Create a heightmap from a point cloud file.
///
/// Loads all the points and uses those that are classified as ground or water to create a heightmap using averages.
pub fn xyz2heightmap(
    fs: &impl FileSystem,
    config: &Config,
    tmpfolder: &Path,
    xyzfilein: &str, // this should be point cloud in
) -> Result<HeightMap, Box<dyn Error>> {
    info!("Generating curves...");

    let scalefactor = config.scalefactor;
    let water_class = config.water_class;

    let mut xmin: f64 = f64::MAX;
    let mut xmax: f64 = f64::MIN;

    let mut ymin: f64 = f64::MAX;
    let mut ymax: f64 = f64::MIN;

    let mut hmin: f64 = f64::MAX;
    let mut hmax: f64 = f64::MIN;

    let xyz_file_in = tmpfolder.join(xyzfilein);
    let mut reader = XyzInternalReader::new(fs.open(&xyz_file_in)?)?;
    while let Some(chunk) = reader.next_chunk()? {
        for r in chunk {
            let x: f64 = r.x;
            let y: f64 = r.y;
            let h: f64 = r.z as f64;

            if xmin > x {
                xmin = x;
            }

            if xmax < x {
                xmax = x;
            }

            if ymin > y {
                ymin = y;
            }

            if ymax < y {
                ymax = y;
            }

            if hmin > h {
                hmin = h;
            }

            if hmax < h {
                hmax = h;
            }
        }
    }
    drop(reader);

    xmin = (xmin / 2.0 / scalefactor).floor() * 2.0 * scalefactor;
    ymin = (ymin / 2.0 / scalefactor).floor() * 2.0 * scalefactor;

    let w: usize = ((xmax - xmin).ceil() / 2.0 / scalefactor) as usize;
    let h: usize = ((ymax - ymin).ceil() / 2.0 / scalefactor) as usize;

    // a two-dimensional vector of (sum, count) pairs for computing averages
    let mut list_alt = Vec2D::new(w + 2, h + 2, (0f64, 0usize));

    let mut reader = XyzInternalReader::new(fs.open(&xyz_file_in)?)?;
    while let Some(chunk) = reader.next_chunk()? {
        for r in chunk {
            if r.classification == 2 || r.classification == water_class {
                let x: f64 = r.x;
                let y: f64 = r.y;
                let h: f64 = r.z as f64;

                let idx_x = ((x - xmin).floor() / 2.0 / scalefactor) as usize;
                let idx_y = ((y - ymin).floor() / 2.0 / scalefactor) as usize;

                let (sum, count) = &mut list_alt[(idx_x, idx_y)];
                *sum += h;
                *count += 1;
            }
        }
    }

    drop(reader);

    let mut avg_alt = Vec2D::new(w + 1, h + 1, f64::NAN);

    for x in 0..w + 1 {
        for y in 0..h + 1 {
            let (sum, count) = &list_alt[(x, y)];

            if *count > 0 {
                avg_alt[(x, y)] = *sum / *count as f64;
            }
        }
    }

    for x in 0..w + 1 {
        for y in 0..h + 1 {
            if avg_alt[(x, y)].is_nan() {
                // interpolate altitude of pixel
                // TODO: optimize to first clasify area then assign values
                let mut i1 = x;
                let mut i2 = x;
                let mut j1 = y;
                let mut j2 = y;

                while i1 > 0 && avg_alt[(i1, y)].is_nan() {
                    i1 -= 1;
                }

                while i2 < w && avg_alt[(i2, y)].is_nan() {
                    i2 += 1;
                }

                while j1 > 0 && avg_alt[(x, j1)].is_nan() {
                    j1 -= 1;
                }

                while j2 < h && avg_alt[(x, j2)].is_nan() {
                    j2 += 1;
                }

                let mut val1 = f64::NAN;
                let mut val2 = f64::NAN;

                if !avg_alt[(i1, y)].is_nan() && !avg_alt[(i2, y)].is_nan() {
                    val1 = ((i2 - x) as f64 * avg_alt[(i1, y)]
                        + (x - i1) as f64 * avg_alt[(i2, y)])
                        / ((i2 - i1) as f64);
                }

                if !avg_alt[(x, j1)].is_nan() && !avg_alt[(x, j2)].is_nan() {
                    val2 = ((j2 - y) as f64 * avg_alt[(x, j1)]
                        + (y - j1) as f64 * avg_alt[(x, j2)])
                        / ((j2 - j1) as f64);
                }

                if !val1.is_nan() && !val2.is_nan() {
                    avg_alt[(x, y)] = (val1 + val2) / 2.0;
                } else if !val1.is_nan() {
                    avg_alt[(x, y)] = val1;
                } else if !val2.is_nan() {
                    avg_alt[(x, y)] = val2;
                }
            }
        }
    }

    for x in 0..w + 1 {
        for y in 0..h + 1 {
            if avg_alt[(x, y)].is_nan() {
                // second round of interpolation of altitude of pixel
                let mut val: f64 = 0.0;
                let mut c = 0;
                for i in 0..3 {
                    let ii: i32 = i - 1;
                    for j in 0..3 {
                        let jj: i32 = j - 1;
                        if y as i32 + jj >= 0 && x as i32 + ii >= 0 {
                            let x_idx = (x as i32 + ii) as usize;
                            let y_idx = (y as i32 + jj) as usize;
                            if x_idx <= w && y_idx <= h && !avg_alt[(x_idx, y_idx)].is_nan() {
                                c += 1;
                                val += avg_alt[(x_idx, y_idx)];
                            }
                        }
                    }
                }
                if c > 0 {
                    avg_alt[(x, y)] = val / c as f64;
                }
            }
        }
    }

    for x in 0..w + 1 {
        for y in 1..h + 1 {
            if avg_alt[(x, y)].is_nan() {
                avg_alt[(x, y)] = avg_alt[(x, y - 1)];
            }
        }
        for yy in 1..h + 1 {
            let y = h - yy;
            if avg_alt[(x, y)].is_nan() {
                avg_alt[(x, y)] = avg_alt[(x, y + 1)];
            }
        }
    }

    xmin += 1.0;
    ymin += 1.0;

    // make sure we do not have any NaNs
    for x in 0..avg_alt.width() {
        for y in 0..avg_alt.height() {
            if avg_alt[(x, y)].is_nan() {
                panic!("heightmap should not have any nans, found NaN at ({x}, {y})");
            }
        }
    }

    let hmap = HeightMap {
        xoffset: xmin,
        yoffset: ymin,
        scale: 2.0 * scalefactor,
        grid: avg_alt.clone(),
    };

    Ok(hmap)
}

/// Creates contour lines from a heightmap.
pub fn heightmap2contours(
    fs: &impl FileSystem,
    tmpfolder: &Path,
    cinterval: f64,
    heightmap: &HeightMap,
    dxffile: &str,
    output_dxf: bool,
) -> Result<(), Box<dyn Error>> {
    let mut avg_alt = heightmap.grid.clone();
    let w = heightmap.grid.width() - 1;
    let h = heightmap.grid.height() - 1;
    let xmin = heightmap.xoffset;
    let ymin = heightmap.yoffset;
    let xmax = heightmap.maxx();
    let ymax = heightmap.maxy();
    let size = heightmap.scale;

    // As per https://github.com/karttapullautin/karttapullautin/discussions/154#discussioncomment-11393907
    // If elevation grid point elavion equals with contour interval steps you will get contour topology issues
    // (crossing/touching contours). This was implemented to avoid that. 0.02 (two centimeters) is just a random
    // small number to avoid that issue, insignificant enough to matter, but big buffer enough to hopefully make
    // it not get back to "bad value" for it getting rounded somewhere. Sure, it could be some fraction of
    // contour interval, but in real world 2 cm is insignificant enough.
    for (_, _, ele) in avg_alt.iter_mut() {
        let temp: f64 = (*ele / cinterval + 0.5).floor() * cinterval;
        if (*ele - temp).abs() < 0.02 {
            if *ele - temp < 0.0 {
                *ele = temp - 0.02;
            } else {
                *ele = temp + 0.02;
            }
        }
    }

    // compute hmin and hmax
    let mut hmin: f64 = f64::MAX;
    let mut hmax: f64 = f64::MIN;
    for (_, _, h) in avg_alt.iter() {
        if h < hmin {
            hmin = h;
        }
        if h > hmax {
            hmax = h;
        }
    }

    let v = cinterval;

    let mut level: f64 = (hmin / v).floor() * v;

    let mut polylines = Vec::<Vec<(f64, f64)>>::new();

    loop {
        if level >= hmax {
            break;
        }

        let mut obj = Vec::<(i64, i64, u8)>::new();
        let mut curves: HashMap<(i64, i64, u8), (i64, i64)> = HashMap::default();

        for i in 1..(w - 1) {
            for j in 2..(h - 1) {
                let mut a = avg_alt[(i, j)];
                let mut b = avg_alt[(i, j + 1)];
                let mut c = avg_alt[(i + 1, j)];
                let mut d = avg_alt[(i + 1, j + 1)];

                if a < level && b < level && c < level && d < level
                    || a > level && b > level && c > level && d > level
                {
                    // skip
                } else {
                    let temp: f64 = (a / v + 0.5).floor() * v;
                    if (a - temp).abs() < 0.05 {
                        if a - temp < 0.0 {
                            a = temp - 0.05;
                        } else {
                            a = temp + 0.05;
                        }
                    }

                    let temp: f64 = (b / v + 0.5).floor() * v;
                    if (b - temp).abs() < 0.05 {
                        if b - temp < 0.0 {
                            b = temp - 0.05;
                        } else {
                            b = temp + 0.05;
                        }
                    }

                    let temp: f64 = (c / v + 0.5).floor() * v;
                    if (c - temp).abs() < 0.05 {
                        if c - temp < 0.0 {
                            c = temp - 0.05;
                        } else {
                            c = temp + 0.05;
                        }
                    }

                    let temp: f64 = (d / v + 0.5).floor() * v;
                    if (d - temp).abs() < 0.05 {
                        if d - temp < 0.0 {
                            d = temp - 0.05;
                        } else {
                            d = temp + 0.05;
                        }
                    }

                    if a < b {
                        if level < b && level > a {
                            let x1: f64 = i as f64;
                            let y1: f64 = j as f64 + (level - a) / (b - a);
                            if level > c {
                                let x2: f64 = i as f64 + (b - level) / (b - c);
                                let y2: f64 = j as f64 + (level - c) / (b - c);
                                check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                            } else if level < c {
                                let x2: f64 = i as f64 + (level - a) / (c - a);
                                let y2: f64 = j as f64;
                                check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                            }
                        }
                    } else if b < a && level < a && level > b {
                        let x1: f64 = i as f64;
                        let y1: f64 = j as f64 + (a - level) / (a - b);
                        if level < c {
                            let x2: f64 = i as f64 + (level - b) / (c - b);
                            let y2: f64 = j as f64 + (c - level) / (c - b);
                            check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                        } else if level > c {
                            let x2: f64 = i as f64 + (a - level) / (a - c);
                            let y2: f64 = j as f64;
                            check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                        }
                    }

                    if a < c {
                        if level < c && level > a {
                            let x1: f64 = i as f64 + (level - a) / (c - a);
                            let y1: f64 = j as f64;
                            if level > b {
                                let x2: f64 = i as f64 + (level - b) / (c - b);
                                let y2: f64 = j as f64 + (c - level) / (c - b);
                                check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                            }
                        }
                    } else if a > c && level < a && level > c {
                        let x1: f64 = i as f64 + (a - level) / (a - c);
                        let y1: f64 = j as f64;
                        if level < b {
                            let x2: f64 = i as f64 + (b - level) / (b - c);
                            let y2: f64 = j as f64 + (level - c) / (b - c);
                            check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                        }
                    }

                    if c < d {
                        if level < d && level > c {
                            let x1: f64 = i as f64 + 1.0;
                            let y1: f64 = j as f64 + (level - c) / (d - c);
                            if level < b {
                                let x2: f64 = i as f64 + (b - level) / (b - c);
                                let y2: f64 = j as f64 + (level - c) / (b - c);
                                check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                            } else if level > b {
                                let x2: f64 = i as f64 + (level - b) / (d - b);
                                let y2: f64 = j as f64 + 1.0;
                                check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                            }
                        }
                    } else if c > d && level < c && level > d {
                        let x1: f64 = i as f64 + 1.0;
                        let y1: f64 = j as f64 + (c - level) / (c - d);
                        if level > b {
                            let x2: f64 = i as f64 + (level - b) / (c - b);
                            let y2: f64 = j as f64 + (c - level) / (c - b);
                            check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                        } else if level < b {
                            let x2: f64 = i as f64 + (b - level) / (b - d);
                            let y2: f64 = j as f64 + 1.0;
                            check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                        }
                    }

                    if d < b {
                        if level < b && level > d {
                            let x1: f64 = i as f64 + (b - level) / (b - d);
                            let y1: f64 = j as f64 + 1.0;
                            if level > c {
                                let x2: f64 = i as f64 + (b - level) / (b - c);
                                let y2: f64 = j as f64 + (level - c) / (b - c);
                                check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                            }
                        }
                    } else if b < d && level < d && level > b {
                        let x1: f64 = i as f64 + (level - b) / (d - b);
                        let y1: f64 = j as f64 + 1.0;
                        if level < c {
                            let x2: f64 = i as f64 + (level - b) / (c - b);
                            let y2: f64 = j as f64 + (c - level) / (c - b);
                            check_obj_in(&mut obj, &mut curves, x1, x2, y1, y2);
                        }
                    }
                }
            }
        }

        for k in obj.iter() {
            if curves.contains_key(k) {
                let mut polyline = Vec::<(f64, f64)>::new();
                let (x, y, _) = *k;
                polyline.push((x as f64 / 100.0, y as f64 / 100.0));

                let mut res = (x, y);

                let (x, y) = *curves.get(k).unwrap();
                polyline.push((x as f64 / 100.0, y as f64 / 100.0));
                curves.remove(k);

                let mut head = (x, y);

                if curves.get(&(head.0, head.1, 1)).is_some_and(|v| *v == res) {
                    curves.remove(&(head.0, head.1, 1));
                }
                if curves.get(&(head.0, head.1, 2)).is_some_and(|v| *v == res) {
                    curves.remove(&(head.0, head.1, 2));
                }
                loop {
                    if curves.get(&(head.0, head.1, 1)).is_some_and(|v| *v != res) {
                        res = head;

                        let (x, y) = *curves.get(&(head.0, head.1, 1)).unwrap();
                        polyline.push((x as f64 / 100.0, y as f64 / 100.0));
                        curves.remove(&(head.0, head.1, 1));

                        head = (x, y);
                        if curves.get(&(head.0, head.1, 1)).is_some_and(|v| *v == res) {
                            curves.remove(&(head.0, head.1, 1));
                        }
                        if curves.get(&(head.0, head.1, 2)).is_some_and(|v| *v == res) {
                            curves.remove(&(head.0, head.1, 2));
                        }
                    } else if curves.get(&(head.0, head.1, 2)).is_some_and(|v| *v != res) {
                        res = head;

                        let (x, y) = *curves.get(&(head.0, head.1, 2)).unwrap();
                        polyline.push((x as f64 / 100.0, y as f64 / 100.0));
                        curves.remove(&(head.0, head.1, 2));

                        head = (x, y);
                        if curves.get(&(head.0, head.1, 1)).is_some_and(|v| *v == res) {
                            curves.remove(&(head.0, head.1, 1));
                        }
                        if curves.get(&(head.0, head.1, 2)).is_some_and(|v| *v == res) {
                            curves.remove(&(head.0, head.1, 2));
                        }
                    } else {
                        polylines.push(polyline);
                        break;
                    }
                }
            }
        }
        level += v;
    }

    // convert the polylines to our internal binary dxf format

    let mut lines = Polylines::new();
    for polyline in polylines.into_iter() {
        lines.push(
            polyline
                .iter()
                .enumerate()
                .filter_map(|(i, (x, y))| {
                    // original logic for some kind of "thinning" of the lines
                    let ii = i + 1;
                    let ldata = polyline.len() - 1;
                    if ii > 5 && ii < ldata - 5 && ldata > 12 && ii % 2 == 0 {
                        return None; // skip this point
                    }

                    // scale the points to world coordinates
                    let x: f64 = x * size + xmin;
                    let y: f64 = y * size + ymin;

                    Some(Point2 { x, y })
                })
                .collect::<Vec<_>>(),
            Classification::ContourSimple,
        );
    }
    let dxf = BinaryDxf::new(Bounds::new(xmin, xmax, ymin, ymax), vec![lines.into()]);

    // write to disk
    dxf.to_fs(fs, tmpfolder.join(dxffile))
        .expect("Cannot write binary dxf file");

    if output_dxf {
        dxf.to_dxf(&mut fs.create(tmpfolder.join(dxffile.strip_suffix(".bin").unwrap()))?)?;
    }

    info!("Done");

    Ok(())
}

fn check_obj_in(
    obj: &mut Vec<(i64, i64, u8)>,
    curves: &mut HashMap<(i64, i64, u8), (i64, i64)>,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
) {
    // convert the coordinates to integers with 2 decimal places for use as keys
    let x1 = (x1 * 100.0).floor() as i64;
    let x2 = (x2 * 100.0).floor() as i64;
    let y1 = (y1 * 100.0).floor() as i64;
    let y2 = (y2 * 100.0).floor() as i64;

    if x1 != x2 || y1 != y2 {
        let key = (x1, y1, 1);
        if !curves.contains_key(&key) {
            curves.insert(key, (x2, y2));
            obj.push(key);
        } else {
            let key = (x1, y1, 2);
            curves.insert(key, (x2, y2));
            obj.push(key);
        }
        let key = (x2, y2, 1);
        if !curves.contains_key(&key) {
            curves.insert(key, (x1, y1));
            obj.push(key);
        } else {
            let key = (x2, y2, 2);
            curves.insert(key, (x1, y1));
            obj.push(key);
        }
    }
}
