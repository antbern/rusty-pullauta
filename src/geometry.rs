//! This mod contains structs for storing and loading different types of geometry, like Polylines
//! and a list of Points.
//!
//! These types also have helpers for exporting them to DXF format.

/// A 2D point
#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point2 {
    /// The x coordinate of this point.
    pub x: f64,
    /// The y coordinate of this point.
    pub y: f64,
}

impl Point2 {
    /// Create a new point from the given coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A collection of points with associated classification. This classification is also used to put
/// the DXF objects into separate layers.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Points {
    points: Vec<Point2>,
    classification: Vec<Classification>,
}

impl Points {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            classification: Vec::new(),
        }
    }

    /// Add a point to this collection.
    pub fn push(&mut self, x: f64, y: f64, class: Classification) {
        self.points.push(Point2::new(x, y));
        self.classification.push(class);
    }

    /// Iterate over the points in this collection.
    pub fn points(&self) -> impl Iterator<Item = (&Point2, &Classification)> {
        self.points.iter().zip(self.classification.iter())
    }
}

/// A collection polylines with associated classification.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Polylines {
    polylines: Vec<Vec<Point2>>, // TODO: flatten to single vector?
    classification: Vec<Classification>,
}

impl Polylines {
    pub fn new() -> Self {
        Self {
            polylines: Vec::new(),
            classification: Vec::new(),
        }
    }

    pub fn push(&mut self, polyline: Vec<Point2>, class: Classification) {
        self.polylines.push(polyline);
        self.classification.push(class);
    }

    pub fn into_iter(self) -> impl Iterator<Item = (Vec<Point2>, Classification)> {
        self.polylines.into_iter().zip(self.classification)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Vec<Point2>, &Classification)> {
        self.polylines.iter().zip(self.classification.iter())
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Geometry {
    Points(Points),
    Polylines(Polylines),
}

impl From<Points> for Geometry {
    fn from(points: Points) -> Self {
        Geometry::Points(points)
    }
}
impl From<Polylines> for Geometry {
    fn from(polylines: Polylines) -> Self {
        Geometry::Polylines(polylines)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BinaryDxf {
    /// the version of the program that created this file, used to detect stale temp files
    version: String,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,

    data: Geometry,
}

impl BinaryDxf {
    pub fn new(xmin: f64, xmax: f64, ymin: f64, ymax: f64, data: Geometry) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            xmin,
            xmax,
            ymin,
            ymax,
            data,
        }
    }

    /// Get the points in this geometry, or [`None`] if does not contain [`Polylines`] data.
    pub fn take_polylines(self) -> Option<Polylines> {
        match self.data {
            Geometry::Polylines(polylines) => Some(polylines),
            Geometry::Points(_) => None,
        }
    }

    /// Serialize this object to a writer.
    pub fn to_writer<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        crate::util::write_object(writer, self)
    }
    /// Read this object from a reader. Returns an error if the version does not match.
    pub fn from_reader<R: std::io::Read>(reader: &mut R) -> anyhow::Result<Self> {
        let object: Self = crate::util::read_object(reader)?;
        anyhow::ensure!(
            object.version == env!("CARGO_PKG_VERSION"),
            "Binary DXF file was created with another version, please remove and recreate"
        );
        Ok(object)
    }

    /// Write this geometry to a DXF file.
    pub fn to_dxf<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        write!(
            writer,
            "  0\r\nSECTION\r\n  2\r\nHEADER\r\n  9\r\n$EXTMIN\r\n 10\r\n{}\r\n 20\r\n{}\r\n  9\r\n$EXTMAX\r\n 10\r\n{}\r\n 20\r\n{}\r\n  0\r\nENDSEC\r\n  0\r\nSECTION\r\n  2\r\nENTITIES\r\n  0\r\n",
            self.xmin, self.ymin, self.xmax, self.ymax
        )?;

        match &self.data {
            Geometry::Points(_points) => {
                todo!()
            }
            Geometry::Polylines(polylines) => {
                for (polyline, class) in polylines.polylines.iter().zip(&polylines.classification) {
                    let layer = class.to_layer();
                    write!(writer, "POLYLINE\r\n 66\r\n1\r\n  8\r\n{layer}\r\n  0\r\n")?;

                    for p in polyline {
                        write!(
                            writer,
                            "VERTEX\r\n  8\r\n{layer}\r\n 10\r\n{}\r\n 20\r\n{}\r\n  0\r\n",
                            p.x, p.y,
                        )?;
                    }
                    write!(writer, "SEQEND\r\n  0\r\n")?;
                }
            }
        }

        writer.write_all("ENDSEC\r\n  0\r\nEOF\r\n".as_bytes())?;
        Ok(())
    }
}

/// Classification used for contour generation
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Classification {
    Contour,
    Cliff2,
    Cliff3,
    Cliff4,
}

impl Classification {
    /// Get the layer name for this classification.
    pub fn to_layer(&self) -> &str {
        match self {
            Self::Contour => "cont",
            Self::Cliff2 => "cliff2",
            Self::Cliff3 => "cliff3",
            Self::Cliff4 => "cliff4",
        }
    }
}
