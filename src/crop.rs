use std::path::Path;

use anyhow::Context;

use crate::geometry::{BinaryDxf, Geometry, Points, Polylines};
use crate::io::fs::FileSystem;

/// Crop the lines that fall outside the bounds by cutting existing lines.
#[allow(clippy::too_many_arguments)]
pub fn polylinebindxfcrop(
    fs: &impl FileSystem,
    input: &Path,
    output: &Path,
    output_dxf: bool,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> anyhow::Result<()> {
    log::debug!("Cropping polylines in binary DXF file: {input:?} to {output:?}");

    // read input file
    let input = BinaryDxf::from_reader(fs, input)?;
    let bounds = input.bounds().clone();

    let output_lines = match input.geometry().first().context("get first geometry")? {
        Geometry::Polylines2(polylines) => {
            crop_lines(polylines, minx, miny, maxx, maxy, |p| (p.x, p.y)).into()
        }
        Geometry::Polylines3(polylines) => {
            crop_lines(polylines, minx, miny, maxx, maxy, |p| (p.x, p.y)).into()
        }
        _ => anyhow::bail!("input file should contain 2D or 3D lines"),
    };

    // write the output (TODO: should we populate the new bounds here or keep the old?)
    let out = BinaryDxf::new(bounds, vec![output_lines]);

    if output_dxf {
        // remove the .bin extension for the DXF output
        out.to_dxf(&mut fs.create(output.with_extension(""))?)?;
    }

    out.to_fs(fs, output)?;

    Ok(())
}

/// Generic inner logic to work with any point type and Classification. Only need to provide an
/// extractor function that will get the x & y components (which is what we are cropping)
fn crop_lines<P: Clone, C: Copy>(
    input_lines: &Polylines<P, C>,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
    xy_fn: impl Fn(&P) -> (f64, f64),
) -> Polylines<P, C> {
    let mut output_lines = Polylines::<_, _>::new();

    for (p, &c) in input_lines.iter() {
        let mut pre = None;
        let mut prex = 0.0;
        let mut prey = 0.0;
        let mut pointcount = 0;
        let mut poly = Vec::with_capacity(p.len());
        for point in p {
            let (valx, valy) = xy_fn(point);
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
            pre = Some(point.clone());
            prex = valx;
            prey = valy;
        }
        if pointcount > 1 {
            output_lines.push(poly, c);
        }
    }
    output_lines
}

/// Removes points that fall outside the provided bounds and writes the remaining points to the
/// output file.
#[allow(clippy::too_many_arguments)]
pub fn pointbindxfcrop(
    fs: &impl FileSystem,
    input: &Path,
    output: &Path,
    output_dxf: bool,
    minx: f64,
    miny: f64,
    maxx: f64,
    maxy: f64,
) -> anyhow::Result<()> {
    log::debug!("Cropping points in binary DXF file: {input:?} to {output:?}");
    // read input file
    let input = BinaryDxf::from_reader(fs, input)?;

    let bounds = input.bounds().clone();
    let Some(Geometry::Points(points)) = input.geometry().first() else {
        anyhow::bail!("input file should contain points");
    };

    // filter all the points
    let mut output_points = Points::with_capacity(points.len());
    for (p, c) in points.iter() {
        if p.x >= minx && p.x <= maxx && p.y >= miny && p.y <= maxy {
            output_points.push(p.clone(), *c);
        }
    }

    // write the output (TODO: should we populate the new bounds here or keep the old?)
    let out = BinaryDxf::new(bounds, vec![output_points.into()]);

    if output_dxf {
        // remove the .bin extension for the DXF output
        out.to_dxf(&mut fs.create(output.with_extension(""))?)?;
    }

    out.to_fs(fs, output)?;

    Ok(())
}
