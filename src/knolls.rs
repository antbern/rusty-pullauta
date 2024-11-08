use image::{GrayImage, Luma};
use imageproc::drawing::draw_line_segment_mut;
use log::info;
use rustc_hash::FxHashMap as HashMap;
use std::error::Error;
use std::io::Write;

use crate::config::Config;
use crate::util::FileProvider;

pub fn dotknolls(config: &Config, provider: &mut FileProvider) -> Result<(), Box<dyn Error>> {
    info!("Identifying dotknolls...");

    let scalefactor = config.scalefactor;

    let xyz_file_in = "xyz_knolls.xyz";

    let mut xstart: f64 = 0.0;
    let mut ystart: f64 = 0.0;
    let mut size: f64 = 0.0;

    let mut i = 0;
    let mut reader = provider.xyz(xyz_file_in);
    while let Some(line) = reader.next().expect("could not read input file") {
        let mut parts = line.split(' ');
        let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();

        if i == 0 {
            xstart = x;
            ystart = y;
        } else if i == 1 {
            size = y - ystart;
        } else {
            break;
        }
        i += 1;
    }
    let mut xmax = 0.0;
    let mut ymax = 0.0;

    let mut reader = provider.xyz(xyz_file_in);
    while let Some(line) = reader.next().expect("could not read input file") {
        let mut parts = line.split(' ');

        // make sure we have at least 2 items
        if let (Some(r0), Some(r1)) = (parts.next(), parts.next()) {
            let x: f64 = r0.parse::<f64>().unwrap();
            let y: f64 = r1.parse::<f64>().unwrap();

            let xx = ((x - xstart) / size).floor();
            let yy = ((y - ystart) / size).floor();

            if xmax < xx {
                xmax = xx;
            }

            if ymax < yy {
                ymax = yy;
            }
        }
    }

    let mut im = GrayImage::from_pixel(
        (xmax * size / scalefactor) as u32,
        (ymax * size / scalefactor) as u32,
        Luma([0xff]),
    );

    let mut f = provider.write("dotknolls.dxf");
    write!(&mut f,
        "  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$EXTMIN\r\n 10\r\n{}\r\n 20\r\n{}\r\n  9\r\n$EXTMAX\r\n 10\r\n{}\r\n 20\r\n{}\r\n  0\r\nENDSEC\r\n  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\n",
        xstart, ystart, xmax * size + xstart, ymax * size + ystart
    ).expect("Cannot write dxf file");

    let data = provider
        .read_to_string("out2.dxf")
        .expect("Can not read input file");
    let data: Vec<&str> = data.split("POLYLINE").collect();

    for (j, rec) in data.iter().enumerate() {
        let mut x = Vec::<f64>::new();
        let mut y = Vec::<f64>::new();
        let mut xline = 0;
        let mut yline = 0;
        if j > 0 {
            let r = rec.split("VERTEX").collect::<Vec<&str>>();
            let apu = r[1];
            let val = apu.split('\n').collect::<Vec<&str>>();
            for (i, v) in val.iter().enumerate() {
                let vt = v.trim();
                if vt == "10" {
                    xline = i + 1;
                }
                if vt == "20" {
                    yline = i + 1;
                }
            }
            for (i, v) in r.iter().enumerate() {
                if i > 0 {
                    let val = v.trim_end().split('\n').collect::<Vec<&str>>();
                    x.push(val[xline].trim().parse::<f64>().unwrap());
                    y.push(val[yline].trim().parse::<f64>().unwrap());
                }
            }
        }
        for i in 1..x.len() {
            draw_line_segment_mut(
                &mut im,
                (
                    ((x[i - 1] - xstart) / scalefactor).floor() as f32,
                    ((y[i - 1] - ystart) / scalefactor).floor() as f32,
                ),
                (
                    ((x[i] - xstart) / scalefactor).floor() as f32,
                    ((y[i] - ystart) / scalefactor).floor() as f32,
                ),
                Luma([0x0]),
            )
        }
    }

    let mut reader = provider.lines("dotknolls.txt");
    while let Some(line) = reader.next().expect("could not read input file") {
        let parts = line.split(' ');
        let r = parts.collect::<Vec<&str>>();
        if r.len() >= 3 {
            let depression: bool = r[0] == "1";
            let x: f64 = r[1].parse::<f64>().unwrap();
            let y: f64 = r[2].parse::<f64>().unwrap();
            let mut ok = true;
            let mut i = (x - xstart) / scalefactor - 3.0;
            while i < (x - xstart) / scalefactor + 4.0 && ok {
                let mut j = (y - ystart) / scalefactor - 3.0;
                while j < (y - ystart) / scalefactor + 4.0 && ok {
                    if (i as u32) >= im.width() || (j as u32) >= im.height() {
                        ok = false;
                        break;
                    }
                    let pix = im.get_pixel(i as u32, j as u32);
                    if pix[0] == 0 {
                        ok = false;
                        break;
                    }
                    j += 1.0;
                }
                i += 1.0;
            }

            let layer = match (ok, depression) {
                (true, true) => "dotknoll",
                (true, false) => "udepression",
                (false, true) => "uglydotknoll",
                (false, false) => "uglyudepression",
            };

            write!(
                &mut f,
                "POINT\r\n  8\r\n{}\r\n 10\r\n{}\r\n 20\r\n{}\r\n 50\r\n0\r\n  0\r\n",
                layer, x, y
            )
            .expect("Can not write to file");
        }
    }

    f.write_all("ENDSEC\r\n  0\r\nEOF\r\n".as_bytes())
        .expect("Can not write to file");
    info!("Done");
    Ok(())
}
pub fn knolldetector(config: &Config, provider: &mut FileProvider) -> Result<(), Box<dyn Error>> {
    info!("Detecting knolls...");
    let scalefactor = config.scalefactor;
    let contour_interval = config.contour_interval;

    let halfinterval = contour_interval / 2.0 * scalefactor;

    let interval = 0.3 * scalefactor;

    let xyz_file_in = "xyz_03.xyz";

    let mut size: f64 = f64::NAN;
    let mut xstart: f64 = f64::NAN;
    let mut ystart: f64 = f64::NAN;

    let mut reader = provider.xyz(xyz_file_in);
    let mut i = 0;
    while let Some(line) = reader.next().expect("could not read input file") {
        let mut parts = line.split(' ');
        let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();

        if i == 0 {
            xstart = x;
            ystart = y;
        } else if i == 1 {
            size = y - ystart;
        } else {
            break;
        }
        i += 1;
    }

    let mut xmax: u64 = u64::MIN;
    let mut ymax: u64 = u64::MIN;
    let mut xmin: u64 = u64::MAX;
    let mut ymin: u64 = u64::MAX;
    let mut xyz: HashMap<(u64, u64), f64> = HashMap::default();
    let mut reader = provider.xyz(xyz_file_in);
    while let Some(line) = reader.next().expect("could not read input file") {
        let mut parts = line.split(' ');
        let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let h: f64 = parts.next().unwrap().parse::<f64>().unwrap();

        let xx = ((x - xstart) / size).floor() as u64;
        let yy = ((y - ystart) / size).floor() as u64;

        xyz.insert((xx, yy), h);

        if xmax < xx {
            xmax = xx;
        }
        if ymax < yy {
            ymax = yy;
        }
        if xmin > xx {
            xmin = xx;
        }
        if ymin > yy {
            ymin = yy;
        }
    }

    let data = provider
        .read_to_string("contours03.dxf")
        .expect("Should have been able to read the file");
    let data: Vec<&str> = data.split("POLYLINE").collect();
    let mut f = provider.write("detected.dxf");
    write!(&mut f,
        "  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$EXTMIN\r\n 10\r\n{}\r\n 20\r\n{}\r\n  9\r\n$EXTMAX\r\n 10\r\n{}\r\n 20\r\n{}\r\n  0\r\nENDSEC\r\n  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\n",
        xmin, ymin, xmax, ymax
    ).expect("Cannot write dxf file");

    let mut heads1: HashMap<String, usize> = HashMap::default();
    let mut heads2: HashMap<String, usize> = HashMap::default();
    let mut heads = Vec::<String>::with_capacity(data.len());
    let mut tails = Vec::<String>::with_capacity(data.len());
    let mut el_x = Vec::<Vec<f64>>::with_capacity(data.len());
    let mut el_y = Vec::<Vec<f64>>::with_capacity(data.len());
    el_x.push(vec![]);
    el_y.push(vec![]);
    heads.push(String::from("-"));
    tails.push(String::from("-"));
    for (j, rec) in data.iter().enumerate() {
        let mut x = Vec::<f64>::new();
        let mut y = Vec::<f64>::new();
        let mut xline = 0;
        let mut yline = 0;
        if j > 0 {
            let r = rec.split("VERTEX").collect::<Vec<&str>>();
            let apu = r[1];
            let val = apu.split('\n').collect::<Vec<&str>>();
            for (i, v) in val.iter().enumerate() {
                let vt = v.trim();
                if vt == "10" {
                    xline = i + 1;
                }
                if vt == "20" {
                    yline = i + 1;
                }
            }
            if r.len() < 201 {
                for (i, v) in r.iter().enumerate() {
                    if i > 0 {
                        let val = v.trim_end().split('\n').collect::<Vec<&str>>();
                        x.push(val[xline].trim().parse::<f64>().unwrap());
                        y.push(val[yline].trim().parse::<f64>().unwrap());
                    }
                }
                let x0 = x.first().unwrap();
                let xl = x.last().unwrap();

                let y0 = y.first().unwrap();
                let yl = y.last().unwrap();

                let head = format!("{}x{}", x0, y0);
                let tail = format!("{}x{}", xl, yl);

                heads.push(head);
                tails.push(tail);

                let head = format!("{}x{}", x0, y0);
                let tail = format!("{}x{}", xl, yl);

                el_x.push(x);
                el_y.push(y);
                if *heads1.get(&head).unwrap_or(&0) == 0 {
                    heads1.insert(head, j);
                } else {
                    heads2.insert(head, j);
                }
                if *heads1.get(&tail).unwrap_or(&0) == 0 {
                    heads1.insert(tail, j);
                } else {
                    heads2.insert(tail, j);
                }
            } else {
                heads.push(String::from("-"));
                tails.push(String::from("-"));
                el_x.push(vec![]);
                el_y.push(vec![]);
            }
        }
    }

    for l in 0..data.len() {
        let mut to_join = 0;
        if !el_x[l].is_empty() {
            let mut end_loop = false;
            while !end_loop {
                let tmp = *heads1.get(&heads[l]).unwrap_or(&0);
                if tmp != 0 && tmp != l && !el_x[tmp].is_empty() {
                    to_join = tmp;
                } else {
                    let tmp = *heads2.get(&heads[l]).unwrap_or(&0);
                    if tmp != 0 && tmp != l && !el_x[tmp].is_empty() {
                        to_join = tmp;
                    } else {
                        let tmp = *heads2.get(&tails[l]).unwrap_or(&0);
                        if tmp != 0 && tmp != l && !el_x[tmp].is_empty() {
                            to_join = tmp;
                        } else {
                            let tmp = *heads1.get(&tails[l]).unwrap_or(&0);
                            if tmp != 0 && tmp != l && !el_x[tmp].is_empty() {
                                to_join = tmp;
                            } else {
                                end_loop = true;
                            }
                        }
                    }
                }
                if !end_loop {
                    if tails[l] == heads[to_join] {
                        let tmp = tails[l].to_string();
                        heads2.insert(tmp, 0);
                        let tmp = tails[l].to_string();
                        heads1.insert(tmp, 0);
                        let mut to_append = el_x[to_join].to_vec();
                        el_x[l].append(&mut to_append);
                        let mut to_append = el_y[to_join].to_vec();
                        el_y[l].append(&mut to_append);
                        let tmp = tails[to_join].to_string();
                        tails[l] = tmp;
                        el_x[to_join].clear();
                        el_y[to_join].clear();
                    } else if tails[l] == tails[to_join] {
                        let tmp = tails[l].to_string();
                        heads2.insert(tmp, 0);
                        let tmp = tails[l].to_string();
                        heads1.insert(tmp, 0);
                        let mut to_append = el_x[to_join].to_vec();
                        to_append.reverse();
                        el_x[l].append(&mut to_append);
                        let mut to_append = el_y[to_join].to_vec();
                        to_append.reverse();
                        el_y[l].append(&mut to_append);
                        let tmp = heads[to_join].to_string();
                        tails[l] = tmp;
                        el_x[to_join].clear();
                        el_y[to_join].clear();
                    } else if heads[l] == tails[to_join] {
                        let tmp = heads[l].to_string();
                        heads2.insert(tmp, 0);
                        let tmp = heads[l].to_string();
                        heads1.insert(tmp, 0);
                        let to_append = el_x[to_join].to_vec();
                        el_x[l].splice(0..0, to_append);
                        let to_append = el_y[to_join].to_vec();
                        el_y[l].splice(0..0, to_append);
                        let tmp = heads[to_join].to_string();
                        heads[l] = tmp;
                        el_x[to_join].clear();
                        el_y[to_join].clear();
                    } else if heads[l] == heads[to_join] {
                        let tmp = heads[l].to_string();
                        heads2.insert(tmp, 0);
                        let tmp = heads[l].to_string();
                        heads1.insert(tmp, 0);
                        let mut to_append = el_x[to_join].to_vec();
                        to_append.reverse();
                        el_x[l].splice(0..0, to_append);
                        let mut to_append = el_y[to_join].to_vec();
                        to_append.reverse();
                        el_y[l].splice(0..0, to_append);
                        let tmp = tails[to_join].to_string();
                        heads[l] = tmp;
                        el_x[to_join].clear();
                        el_y[to_join].clear();
                    }
                }
            }
        }
    }

    let mut elevation: HashMap<u64, f64> = HashMap::default();
    for l in 0..data.len() {
        let mut skip = false;
        let el_x_len = el_x[l].len();
        if el_x_len > 0 {
            if el_x_len > 121 {
                skip = true;
                el_x[l].clear();
                el_y[l].clear();
            }
            if el_x_len < 9 {
                let mut p = 0;
                let mut dist = 0.0;
                while p < el_x_len - 1 {
                    dist += ((el_x[l][p] - el_x[l][p + 1]).powi(2)
                        + (el_y[l][p] - el_y[l][p + 1]).powi(2))
                    .sqrt();
                    p += 1;
                }
                if dist < 5.0 || el_x_len < 3 {
                    skip = true;
                    el_x[l].clear();
                    el_y[l].clear();
                }
            }
            if el_x[l].first() != el_x[l].last() || el_y[l].first() != el_y[l].last() {
                skip = true;
                el_x[l].clear();
                el_y[l].clear();
            }
            if !skip
                && el_x_len < 122
                && el_x[l].first() == el_x[l].last()
                && el_y[l].first() == el_y[l].last()
            {
                let tailx = *el_x[l].first().unwrap();
                let mut xl = el_x[l].to_vec();
                xl.push(tailx);
                let taily = *el_y[l].first().unwrap();
                let mut yl = el_y[l].to_vec();
                yl.push(taily);
                let mut mm = ((el_x_len as f64 / 3.0).floor() - 1.0) as i32;
                if mm < 0 {
                    mm = 0;
                }
                let mut m = mm as usize;
                let mut h = 0.0;
                while m < xl.len() {
                    let xm = xl[m];
                    let ym = yl[m];
                    let xo = (xm - xstart) / size;
                    let yo = (ym - ystart) / size;
                    if xo == xo.floor() {
                        let h1 = *xyz
                            .get(&(xo.floor() as u64, yo.floor() as u64))
                            .unwrap_or(&0.0);
                        let h2 = *xyz
                            .get(&(xo.floor() as u64, yo.floor() as u64 + 1))
                            .unwrap_or(&0.0);
                        h = h1 * (yo.floor() + 1.0 - yo) + h2 * (yo - yo.floor());
                        h = (h / interval + 0.5).floor() * interval;
                        break;
                    } else if m < (el_x_len - 3) && yo == yo.floor() {
                        let h1 = *xyz
                            .get(&(xo.floor() as u64, yo.floor() as u64))
                            .unwrap_or(&0.0);
                        let h2 = *xyz
                            .get(&(xo.floor() as u64 + 1, yo.floor() as u64))
                            .unwrap_or(&0.0);
                        h = h1 * (xo.floor() + 1.0 - xo) + h2 * (xo - xo.floor());
                        h = (h / interval + 0.5).floor() * interval;
                    }
                    m += 1;
                }
                elevation.insert(l as u64, h);

                let mut mm = ((el_x_len as f64 / 3.0).floor() - 1.0) as i32;
                if mm < 0 {
                    mm = 0;
                }
                let mut m = mm as usize;
                let mut xa = xl[m];
                let mut ya = yl[m];
                while m < xl.len() {
                    let xm = xl[m];
                    let ym = yl[m];
                    let xo = (xm - xstart) / size;
                    let yo = (ym - ystart) / size;
                    if m < xl.len() - 3 && yo == yo.floor() && xo != xo.floor() {
                        xa = xo.floor() * size + xstart;
                        ya = ym.floor();
                        break;
                    }
                    m += 1;
                }
                let h_center = *xyz
                    .get(&(
                        ((xa - xstart) / size).floor() as u64,
                        ((ya - ystart) / size).floor() as u64,
                    ))
                    .unwrap_or(&0.0);
                let mut hit = 0;
                let xtest = ((xa - xstart) / size).floor() * size + xstart + 0.000000001;
                let ytest = ((ya - ystart) / size).floor() * size + ystart + 0.000000001;

                let mut n = 0;
                let mut y0 = 0.0;
                let mut x0 = 0.0;
                while n < (el_x_len - 1) {
                    let x1 = el_x[l][n];
                    let y1 = el_y[l][n];
                    if n > 0
                        && ((y0 <= ytest && ytest < y1) || (y1 <= ytest && ytest < y0))
                        && (xtest < ((x1 - x0) * (ytest - y0) / (y1 - y0) + x0))
                    {
                        hit += 1;
                    }
                    n += 1;
                    x0 = x1;
                    y0 = y1;
                }

                if (h_center < h) && (hit % 2 == 1) || (h_center > h) && (hit % 2 != 1) {
                    skip = true;
                    el_x[l].clear();
                    el_y[l].clear();
                }
            }
        }
        if skip {
            el_x[l].clear();
            el_y[l].clear();
        }
    }

    struct Head {
        id: u64,
        xtest: f64,
        ytest: f64,
    }
    let mut heads = Vec::<Head>::new();
    for l in 0..data.len() {
        if !el_x[l].is_empty() {
            if el_x[l].first() == el_x[l].last() && el_y[l].first() == el_y[l].last() {
                heads.push(Head {
                    id: l as u64,
                    xtest: el_x[l][0],
                    ytest: el_y[l][0],
                });
            } else {
                el_x[l].clear();
                el_y[l].clear();
            }
        }
    }
    struct Top {
        id: u64,
        xtest: f64,
        ytest: f64,
    }
    let mut tops = Vec::<Top>::new();
    struct BoundingBox {
        minx: f64,
        maxx: f64,
        miny: f64,
        maxy: f64,
    }
    let mut bb: HashMap<usize, BoundingBox> = HashMap::default();
    for l in 0..data.len() {
        let mut skip = false;
        if !el_x[l].is_empty() {
            let mut x = el_x[l].to_vec();
            let tailx = *el_x[l].first().unwrap();
            x.push(tailx);

            let mut y = el_y[l].to_vec();
            let taily = *el_y[l].first().unwrap();
            y.push(taily);

            let mut minx = f64::MAX;
            let mut miny = f64::MAX;
            let mut maxx = f64::MIN;
            let mut maxy = f64::MIN;

            for k in 0..x.len() {
                if x[k] > maxx {
                    maxx = x[k]
                }
                if x[k] < minx {
                    minx = x[k]
                }
                if y[k] > maxy {
                    maxy = y[k]
                }
                if y[k] < miny {
                    miny = y[k]
                }
            }
            bb.insert(
                l,
                BoundingBox {
                    minx,
                    maxx,
                    miny,
                    maxy,
                },
            );

            for head in heads.iter() {
                let &Head { id, xtest, ytest } = head;

                if !skip
                    && *elevation.get(&id).unwrap() > *elevation.get(&(l as u64)).unwrap()
                    && id != (l as u64)
                    && xtest < maxx
                    && xtest > minx
                    && ytest < maxy
                    && ytest > miny
                {
                    let mut hit = 0;
                    let mut n = 0;
                    let mut x0 = 0.0;
                    let mut y0 = 0.0;
                    while n < x.len() {
                        let x1 = x[n];
                        let y1 = y[n];

                        if n > 0
                            && ((y0 <= ytest && ytest < y1) || (y1 <= ytest && ytest < y0))
                            && (xtest < ((x1 - x0) * (ytest - y0) / (y1 - y0) + x0))
                        {
                            hit += 1;
                        }
                        x0 = x1;
                        y0 = y1;
                        n += 1;
                    }
                    if hit % 2 == 1 {
                        skip = true;
                    }
                }
            }
            if !skip {
                tops.push(Top {
                    id: l as u64,
                    xtest: x[0],
                    ytest: y[0],
                });
            }
        }
    }
    struct Candidate {
        id: u64,
        xtest: f64,
        ytest: f64,
        topid: u64,
    }
    let mut canditates = Vec::<Candidate>::new();

    for l in 0..data.len() {
        let mut skip = true;
        if !el_x[l].is_empty() {
            let mut x = el_x[l].to_vec();
            let tailx = *el_x[l].first().unwrap();
            x.push(tailx);

            let mut y = el_y[l].to_vec();
            let taily = *el_y[l].first().unwrap();
            y.push(taily);

            let &BoundingBox {
                minx,
                maxx,
                miny,
                maxy,
            } = bb.get(&l).unwrap();

            let mut topid = 0;
            for head in tops.iter() {
                let &Top { id, xtest, ytest } = head;
                let ll = l as u64;

                if *elevation.get(&ll).unwrap() < (*elevation.get(&id).unwrap() - 0.1)
                    && *elevation.get(&ll).unwrap() > (*elevation.get(&id).unwrap() - 4.6)
                    && skip
                    && xtest < maxx
                    && xtest > minx
                    && ytest < maxy
                    && ytest > miny
                {
                    let mut hit = 0;
                    let mut n = 0;

                    let mut x0 = 0.0;
                    let mut y0 = 0.0;
                    while n < x.len() {
                        let x1 = x[n];
                        let y1 = y[n];

                        if n > 0
                            && ((y0 <= ytest && ytest < y1) || (y1 <= ytest && ytest < y0))
                            && (xtest < ((x1 - x0) * (ytest - y0) / (y1 - y0) + x0))
                        {
                            hit += 1;
                        }
                        x0 = x1;
                        y0 = y1;

                        n += 1;
                    }
                    if hit % 2 == 1 {
                        skip = false;
                        topid = id;
                    }
                }
            }
            if !skip {
                canditates.push(Candidate {
                    id: l as u64,
                    xtest: x[0],
                    ytest: y[0],
                    topid,
                });
            } else {
                el_x[l].clear();
                el_y[l].clear();
            }
        }
    }

    let mut best: HashMap<u64, u64> = HashMap::default();
    let mut mov: HashMap<u64, f64> = HashMap::default();

    for head in canditates.iter() {
        let &Candidate { id, topid, .. } = head;
        let el = *elevation.get(&id).unwrap();
        let test = (el / halfinterval + 1.0).floor() * halfinterval - el;

        if !best.contains_key(&topid) {
            best.insert(topid, id);
            mov.insert(id, test);
        } else {
            let tid = *best.get(&topid).unwrap();
            if *mov.get(&tid).unwrap() < 1.75
                && (*elevation.get(&topid).unwrap() - *elevation.get(&tid).unwrap() - 0.6).abs()
                    < 0.2
            {
                // no action
            } else if *mov.get(&tid).unwrap() > test {
                best.insert(topid, id);
                mov.insert(id, test);
            }
        }
    }
    let mut new_candidates = Vec::<Candidate>::new();
    for head in canditates.iter() {
        let &Candidate {
            id,
            xtest,
            ytest,
            topid,
        } = head;

        let x = el_x[id as usize].to_vec();
        if *best.get(&topid).unwrap() == id
            && (x.len() < 13
                || (*elevation.get(&topid).unwrap() > (*elevation.get(&id).unwrap() + 0.45)
                    || (*elevation.get(&id).unwrap()
                        - 2.5 * (*elevation.get(&id).unwrap() / 2.5).floor())
                        > 0.45))
        {
            new_candidates.push(Candidate {
                id,
                xtest,
                ytest,
                topid,
            });
        } else {
            el_x[id as usize].clear();
            el_y[id as usize].clear();
        }
    }

    let canditates = new_candidates;

    let mut file_pins = provider.write("pins.txt");

    for l in 0..data.len() {
        let mut skip = false;
        let ll = l as u64;
        let mut ltopid = 0;
        if !el_x[l].is_empty() {
            let mut x = el_x[l].to_vec();
            let tailx = *el_x[l].first().unwrap();
            x.push(tailx);

            let mut y = el_y[l].to_vec();
            let taily = *el_y[l].first().unwrap();
            y.push(taily);

            let &BoundingBox {
                minx,
                maxx,
                miny,
                maxy,
            } = bb.get(&l).unwrap();

            for head in canditates.iter() {
                let &Candidate {
                    id,
                    xtest,
                    ytest,
                    topid,
                } = head;

                ltopid = topid;
                if id != ll && !skip && xtest < maxx && xtest > minx && ytest < maxy && ytest > miny
                {
                    let mut hit = 0;
                    let mut n = 0;

                    let mut x0 = 0.0;
                    let mut y0 = 0.0;
                    while n < x.len() {
                        let x1 = x[n];
                        let y1 = y[n];

                        if n > 0
                            && ((y0 <= ytest && ytest < y1) || (y1 <= ytest && ytest < y0))
                            && (xtest < ((x1 - x0) * (ytest - y0) / (y1 - y0) + x0))
                        {
                            hit += 1;
                        }
                        x0 = x1;
                        y0 = y1;

                        n += 1;
                    }
                    if hit % 2 == 1 {
                        skip = true;
                    }
                }
            }

            if !skip {
                f.write_all("POLYLINE\r\n 66\r\n1\r\n  8\r\n1010\r\n  0\r\n".as_bytes())
                    .expect("Can not write to file");
                let mut xa = 0.0;
                let mut ya = 0.0;
                for k in 0..x.len() {
                    xa += x[k];
                    ya += y[k];
                }
                let xlen = x.len() as f64;
                xa /= xlen;
                ya /= xlen;

                write!(
                    &mut file_pins,
                    "{},{},{},{},{},{},{},{}\r\n",
                    x[0],
                    y[0],
                    *elevation.get(&ll).unwrap(),
                    xa,
                    ya,
                    *elevation.get(&ltopid).unwrap(),
                    x.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(" "),
                    y.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                )
                .expect("Could not write to file");

                for k in 0..x.len() {
                    write!(
                        &mut f,
                        "VERTEX\r\n  8\r\n1010\r\n 10\r\n{}\r\n 20\r\n{}\r\n  0\r\n",
                        x[k], y[k]
                    )
                    .expect("Can not write to file");
                }
                f.write_all("SEQEND\r\n  0\r\n".as_bytes())
                    .expect("Can not write to file");
            } else {
                el_x[l].clear();
                el_y[l].clear();
            }
        }
    }
    f.write_all("ENDSEC\r\n  0\r\nEOF\r\n".as_bytes())
        .expect("Can not write to file");

    info!("Done");
    Ok(())
}

pub fn xyzknolls(config: &Config, provider: &mut FileProvider) -> Result<(), Box<dyn Error>> {
    info!("Identifying knolls...");
    let scalefactor = config.scalefactor;
    let contour_interval = config.contour_interval;

    let interval = contour_interval / 2.0 * scalefactor;

    let xyz_file_in = "xyz_03.xyz";

    let mut xstart: f64 = 0.0;
    let mut ystart: f64 = 0.0;
    let mut size: f64 = 0.0;

    let mut reader = provider.xyz(xyz_file_in);
    let mut i = 0;
    while let Some(line) = reader.next().expect("could not read input file") {
        let mut parts = line.split(' ');
        let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();

        if i == 0 {
            xstart = x;
            ystart = y;
        } else if i == 1 {
            size = y - ystart;
        } else {
            break;
        }
        i += 1;
    }
    let mut xmax: u64 = 0;
    let mut ymax: u64 = 0;
    let mut xyz: HashMap<(u64, u64), f64> = HashMap::default();
    let mut reader = provider.xyz(xyz_file_in);
    while let Some(line) = reader.next().expect("could not read input file") {
        let mut parts = line.split(' ');
        let x: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let y: f64 = parts.next().unwrap().parse::<f64>().unwrap();
        let h: f64 = parts.next().unwrap().parse::<f64>().unwrap();

        let xx = ((x - xstart) / size).floor() as u64;
        let yy = ((y - ystart) / size).floor() as u64;
        xyz.insert((xx, yy), h);
        if xmax < xx {
            xmax = xx;
        }
        if ymax < yy {
            ymax = yy;
        }
    }

    let mut xyz2: HashMap<(u64, u64), f64> = xyz.clone();
    for i in 2..(xmax as usize - 1) {
        for j in 2..(ymax as usize - 1) {
            let mut low = f64::MAX;
            let mut high = f64::MIN;
            let mut val = 0.0;
            let mut count = 0;
            for ii in (i - 2)..(i + 3) {
                for jj in (j - 2)..(j + 3) {
                    let tmp = *xyz.get(&(ii as u64, jj as u64)).unwrap_or(&0.0);
                    if tmp < low {
                        low = tmp;
                    }
                    if tmp > high {
                        high = tmp;
                    }
                    count += 1;
                    val += tmp;
                }
            }
            let steepness = high - low;
            if steepness < 1.25 {
                let tmp = (1.25 - steepness) * (val - low - high) / (count as f64 - 2.0) / 1.25
                    + steepness * (*xyz2.get(&(i as u64, j as u64)).unwrap_or(&0.0)) / 1.25;
                xyz2.insert((i as u64, j as u64), tmp);
            }
        }
    }

    struct Pin {
        xx: f64,
        yy: f64,
        ele: f64,
        ele2: f64,
        xlist: Vec<f64>,
        ylist: Vec<f64>,
    }
    let mut pins: Vec<Pin> = Vec::new();

    let pins_file_in = "pins.txt";
    if provider.exists(pins_file_in) {
        let mut reader = provider.lines(pins_file_in);
        while let Some(line) = reader.next().expect("could not read file") {
            let mut r = line.trim().split(',');
            let ele = r.nth(2).unwrap().parse::<f64>().unwrap();
            let xx = r.next().unwrap().parse::<f64>().unwrap();
            let yy = r.next().unwrap().parse::<f64>().unwrap();
            let ele2 = r.next().unwrap().parse::<f64>().unwrap();
            let xlist = r.next().unwrap();
            let ylist = r.next().unwrap();
            let mut x: Vec<f64> = xlist
                .split(' ')
                .map(|s| s.parse::<f64>().unwrap())
                .collect();
            let mut y: Vec<f64> = ylist
                .split(' ')
                .map(|s| s.parse::<f64>().unwrap())
                .collect();
            x.push(x[0]);
            y.push(y[0]);

            pins.push(Pin {
                xx,
                yy,
                ele,
                ele2,
                xlist: x,
                ylist: y,
            });
        }
    }

    // compute closest distance from each pin to another pin
    let mut dist: HashMap<usize, f64> = HashMap::default();
    for (l, pin) in pins.iter().enumerate() {
        let mut min = f64::MAX;
        let xx = ((pin.xx - xstart) / size).floor();
        let yy = ((pin.yy - ystart) / size).floor();
        for (k, pin2) in pins.iter().enumerate() {
            if k == l {
                continue;
            }

            let xx2 = ((pin2.xx - xstart) / size).floor();
            let yy2 = ((pin2.yy - ystart) / size).floor();
            let mut dis = (xx2 - xx).abs();
            let disy = (yy2 - yy).abs();
            if disy > dis {
                dis = disy;
            }
            if dis < min {
                min = dis;
            }
        }
        dist.insert(l, min);
    }

    for (l, line) in pins.into_iter().enumerate() {
        let Pin {
            xx,
            yy,
            ele,
            ele2,
            xlist: mut x,
            ylist: mut y,
        } = line;

        let elenew = ((ele - 0.09) / interval + 1.0).floor() * interval;
        let mut move1 = elenew - ele + 0.15;
        let mut move2 = move1 * 0.4;
        if move1 > 0.66 * interval {
            move2 = move1 * 0.6;
        }
        if move1 < 0.25 * interval {
            move2 = 0.0;
            move1 += 0.3;
        }
        move1 += 0.5;
        if ele2 + move1 > ((ele - 0.09) / interval + 2.0).floor() * interval {
            move1 -= 0.4;
        }
        if elenew - ele > 1.5 * scalefactor && x.len() > 21 {
            for k in 0..x.len() {
                x[k] = xx + (x[k] - xx) * 0.8;
                y[k] = yy + (y[k] - yy) * 0.8;
            }
        }
        let mut touched: HashMap<String, bool> = HashMap::default();
        let mut minx = u64::MAX;
        let mut miny = u64::MAX;
        let mut maxx = u64::MIN;
        let mut maxy = u64::MIN;
        for k in 0..x.len() {
            x[k] = ((x[k] - xstart) / size + 0.5).floor();
            y[k] = ((y[k] - ystart) / size + 0.5).floor();
            let xk = x[k] as u64;
            let yk = y[k] as u64;
            if xk > maxx {
                maxx = xk;
            }
            if yk > maxy {
                maxy = yk;
            }
            if xk < minx {
                minx = xk;
            }
            if yk < miny {
                miny = yk;
            }
        }

        let xx = ((xx - xstart) / size).floor();
        let yy = ((yy - ystart) / size).floor();

        let mut x0 = 0.0;
        let mut y0 = 0.0;

        for ii in minx as usize..(maxx as usize + 1) {
            for jj in miny as usize..(maxy as usize + 1) {
                let mut hit = 0;
                let xtest = ii as f64;
                let ytest = jj as f64;
                for n in 0..x.len() {
                    let x1 = x[n];
                    let y1 = y[n];
                    if n > 1
                        && ((y0 <= ytest && ytest < y1) || (y1 <= ytest && ytest < y0))
                        && xtest < (x1 - x0) * (ytest - y0) / (y1 - y0) + x0
                    {
                        hit += 1;
                    }
                    x0 = x1;
                    y0 = y1;
                }
                if hit % 2 == 1 {
                    let tmp = *xyz2.get(&(ii as u64, jj as u64)).unwrap_or(&0.0) + move1;
                    xyz2.insert((ii as u64, jj as u64), tmp);
                    let coords = format!("{}_{}", ii, jj);
                    touched.insert(coords, true);
                }
            }
        }
        let mut range = *dist.get(&l).unwrap_or(&0.0) * 0.8 - 1.0;
        range = range.clamp(1.0, 12.0);

        for iii in 0..((range * 2.0 + 1.0) as usize) {
            for jjj in 0..((range * 2.0 + 1.0) as usize) {
                let ii: f64 = xx - range + iii as f64;
                let jj: f64 = yy - range + jjj as f64;
                if ii > 0.0 && ii < xmax as f64 + 1.0 && jj > 0.0 && jj < ymax as f64 + 1.0 {
                    let coords = format!("{}_{}", ii, jj);
                    if !*touched.get(&coords).unwrap_or(&false) {
                        let tmp = *xyz2
                            .get(&(ii.floor() as u64, jj.floor() as u64))
                            .unwrap_or(&0.0)
                            + (range - (xx - ii).abs()) / range * (range - (yy - jj).abs()) / range
                                * move2;
                        xyz2.insert((ii.floor() as u64, jj.floor() as u64), tmp);
                    }
                }
            }
        }
    }

    let mut f2 = provider.write("xyz_knolls.xyz");

    let mut reader = provider.xyz(xyz_file_in);
    while let Some(line) = reader.next().expect("could not read input file") {
        let parts = line.split(' ');
        let mut r = parts.collect::<Vec<&str>>();
        let x: f64 = r[0].parse::<f64>().unwrap();
        let y: f64 = r[1].parse::<f64>().unwrap();
        let mut h = *xyz2
            .get(&(
                ((x - xstart) / size).floor() as u64,
                ((y - ystart) / size).floor() as u64,
            ))
            .unwrap_or(&0.0);
        let tmp = (h / interval + 0.5).floor() * interval;
        if (tmp - h).abs() < 0.02 {
            if h - tmp < 0.0 {
                h = tmp - 0.02;
            } else {
                h = tmp + 0.02;
            }
        }
        let new_val = format!("{}", h);
        r[2] = &new_val;
        let out = r.join(" ");
        f2.write_all(out.as_bytes()).expect("cannot write to file");
        f2.write_all("\n".as_bytes()).expect("cannot write to file");
    }

    info!("Done");
    Ok(())
}
