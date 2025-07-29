use std::error::Error;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use crate::geometry::{BinaryDxf, Classification, Geometry, Point2, Points, Polylines};
use crate::io::fs::FileSystem;

/// Crop the lines that fall outside the bounds by cutting existing lines.
pub fn polylinebindxfcrop(
    fs: &impl FileSystem,
    input: &Path,
    output: &Path,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> anyhow::Result<()> {
    log::debug!("Cropping polylines in binary DXF file: {input:?} to {output:?}");

    // read input file
    let input = BinaryDxf::from_reader(&mut BufReader::new(fs.open(input)?))?;
    let bounds = input.bounds().clone();
    let Geometry::Polylines2(points) = input.take_geometry() else {
        anyhow::bail!("input file should contain 2D lines");
    };

    let mut output_lines = Polylines::<Point2, Classification>::new();

    for (p, c) in points.into_iter() {
        let mut pre = None;
        let mut prex = 0.0;
        let mut prey = 0.0;
        let mut pointcount = 0;
        let mut poly: Vec<Point2> = Vec::with_capacity(p.len());
        for point in p {
            let valx = point.x;
            let valy = point.y;
            if valx >= minx && valx <= maxx && valy >= miny && valy <= maxy {
                if let Some(pre) = pre
                    && pointcount == 0
                    && (prex < minx || prey < miny)
                {
                    poly.push(pre);
                    pointcount += 1;
                }
                poly.push(point.clone());
                pointcount += 1;
            } else if pointcount > 1 {
                if valx < minx || valy < miny {
                    poly.push(point.clone());
                }

                output_lines.push(poly, c);
                poly = Vec::new();
                pointcount = 0;
            }
            pre = Some(point);
            prex = valx;
            prey = valy;
        }
        if pointcount > 1 {
            output_lines.push(poly, c);
        }
    }

    // write the output (TODO: should we populate the new bounds here or keep the old?)
    let out = BinaryDxf::new(bounds, output_lines.into());
    out.to_writer(&mut BufWriter::new(fs.create(output)?))?;

    out.to_dxf(&mut BufWriter::new(
        fs.create(output.with_extension("dxf"))?,
    ))?;

    Ok(())
}

/// Original version that operates directly on DXF formatted files. Kept so that the CLI part can
/// use it. Consider removing if not needed.
pub fn polylinedxfcrop(
    fs: &impl FileSystem,
    input: &Path,
    output: &Path,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> Result<(), Box<dyn Error>> {
    log::debug!("Cropping polylines in DXF file: {input:?} to {output:?}");
    let data = fs
        .read_to_string(input)
        .expect("Should have been able to read the file");
    let data: Vec<&str> = data.split("POLYLINE").collect();
    let dxfhead = data[0];
    let mut out = String::new();
    out.push_str(dxfhead);
    for (j, rec) in data.iter().enumerate() {
        let mut poly = String::new();
        let mut pre = "";
        let mut prex = 0.0;
        let mut prey = 0.0;
        let mut pointcount = 0;
        if j > 0 {
            if let Some((head, rec2)) = rec.split_once("VERTEX") {
                let r: Vec<&str> = rec2.split("VERTEX").collect();
                poly.push_str(&format!("POLYLINE{head}"));
                for apu in r.iter() {
                    let (apu2, _notused) = apu.split_once("SEQEND").unwrap_or((apu, ""));
                    let val: Vec<&str> = apu2.split('\n').collect();
                    let mut xline = 0;
                    let mut yline = 0;
                    for (i, v) in val.iter().enumerate() {
                        let vt = v.trim_end();
                        if vt == " 10" {
                            xline = i + 1;
                        }
                        if vt == " 20" {
                            yline = i + 1;
                        }
                    }
                    let valx = val[xline].trim().parse::<f64>().unwrap_or(0.0);
                    let valy = val[yline].trim().parse::<f64>().unwrap_or(0.0);
                    if valx >= minx && valx <= maxx && valy >= miny && valy <= maxy {
                        if !pre.is_empty() && pointcount == 0 && (prex < minx || prey < miny) {
                            poly.push_str(&format!("VERTEX{pre}"));
                            pointcount += 1;
                        }
                        poly.push_str(&format!("VERTEX{apu}"));
                        pointcount += 1;
                    } else if pointcount > 1 {
                        if valx < minx || valy < miny {
                            poly.push_str(&format!("VERTEX{apu}"));
                        }
                        if !poly.contains("SEQEND") {
                            poly.push_str("SEQEND\r\n0\r\n");
                        }
                        out.push_str(&poly);
                        poly = format!("POLYLINE{head}");
                        pointcount = 0;
                    }
                    pre = apu2;
                    prex = valx;
                    prey = valy;
                }
                if !poly.contains("SEQEND") {
                    poly.push_str("SEQEND\r\n  0\r\n");
                }
                if pointcount > 1 {
                    out.push_str(&poly);
                }
            }
        }
    }

    if !out.contains("EOF") {
        out.push_str("ENDSEC\r\n  0\r\nEOF\r\n");
    }
    let fp = fs.create(output).expect("Unable to create file");
    let mut fp = BufWriter::new(fp);
    fp.write_all(out.as_bytes()).expect("Unable to write file");
    Ok(())
}

/// Removes points that fall outside the provided bounds and writes the remaining points to the
/// output file.
pub fn pointbindxfcrop(
    fs: &impl FileSystem,
    input: &Path,
    output: &Path,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> anyhow::Result<()> {
    log::debug!("Cropping points in binary DXF file: {input:?} to {output:?}");
    // read input file
    let input = BinaryDxf::from_reader(&mut BufReader::new(fs.open(input)?))?;

    let bounds = input.bounds().clone();
    let Geometry::Points(points) = input.take_geometry() else {
        anyhow::bail!("input file should contain points");
    };

    // filter all the points
    let mut output_points = Points::with_capacity(points.len());
    for (p, c) in points.into_iter() {
        if p.x >= minx && p.x <= maxx && p.y >= miny && p.y <= maxy {
            output_points.push(p, c);
        }
    }

    // write the output (TODO: should we populate the new bounds here or keep the old?)
    let out = BinaryDxf::new(bounds, output_points.into());
    out.to_writer(&mut BufWriter::new(fs.create(output)?))?;

    Ok(())
}

/// Original version that operates directly on DXF formatted files. Kept so that the CLI part can
/// use it. Consider removing if not needed.
pub fn pointdxfcrop(
    fs: &impl FileSystem,
    input: &Path,
    output: &Path,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> Result<(), Box<dyn Error>> {
    log::debug!("Cropping points in DXF file: {input:?} to {output:?}");
    let data = fs
        .read_to_string(input)
        .expect("Should have been able to read the file");
    let mut data: Vec<&str> = data.split("POINT").collect();
    let dxfhead = data[0];

    let fp = fs.create(output).expect("Unable to create file");
    let mut fp = BufWriter::new(fp);

    fp.write_all(dxfhead.as_bytes())
        .expect("Could not write file");

    let (d2, ending) = data[data.len() - 1]
        .split_once("ENDSEC")
        .unwrap_or((data[data.len() - 1], ""));
    let last_idx = data.len() - 1;
    data[last_idx] = d2;
    for (j, rec) in data.iter().enumerate() {
        if j > 0 {
            let val: Vec<&str> = rec.split('\n').collect();
            let val4 = val[4].trim().parse::<f64>().unwrap_or(0.0);
            let val6 = val[6].trim().parse::<f64>().unwrap_or(0.0);
            if val4 >= minx && val4 <= maxx && val6 >= miny && val6 <= maxy {
                write!(fp, "POINT{rec}").expect("Could not write file");
            }
        }
    }
    write!(fp, "ENDSEC{ending}").expect("Could not write file");
    Ok(())
}
