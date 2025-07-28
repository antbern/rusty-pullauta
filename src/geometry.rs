//! This mod contains structs for storing and loading different types of geometry, like Polylines
//! and a list of Points.
//!
//! These types also have helpers for exporting them to DXF format.

/// Needs to be implemented by any classification type that can be converted to a DXF layer name.
pub trait ClassificationToLayer {
    fn to_layer(&self) -> &str;
}

/// A collection of points with associated classification. This classification is also used to put
/// the DXF objects into separate layers.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Points<C> {
    points: Vec<(f64, f64)>,
    classification: Vec<C>,
}

impl<C> Points<C> {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            classification: Vec::new(),
        }
    }

    /// Add a point to this collection.
    pub fn push(&mut self, x: f64, y: f64, class: C) {
        self.points.push((x, y));
        self.classification.push(class);
    }

    /// Iterate over the points in this collection.
    pub fn points(&self) -> impl Iterator<Item = (&(f64, f64), &C)> {
        self.points.iter().zip(self.classification.iter())
    }
}

/// A collection polylines with associated classification.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Polylines<C> {
    polylines: Vec<Vec<(f64, f64)>>, // TODO: flatten to single vector?
    classification: Vec<C>,
}

impl<C> Polylines<C> {
    pub fn new() -> Self {
        Self {
            polylines: Vec::new(),
            classification: Vec::new(),
        }
    }

    pub fn push(&mut self, polyline: Vec<(f64, f64)>, class: C) {
        self.polylines.push(polyline);
        self.classification.push(class);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum Geometry<C> {
    Points(Points<C>),
    Polylines(Polylines<C>),
}

impl<C> From<Points<C>> for Geometry<C> {
    fn from(points: Points<C>) -> Self {
        Geometry::Points(points)
    }
}
impl<C> From<Polylines<C>> for Geometry<C> {
    fn from(polylines: Polylines<C>) -> Self {
        Geometry::Polylines(polylines)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BinaryDxf<C> {
    /// the version of the program that created this file, used to detect stale temp files
    version: String,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,

    data: Geometry<C>,
}

impl<C> BinaryDxf<C> {
    pub fn new(xmin: f64, xmax: f64, ymin: f64, ymax: f64, data: Geometry<C>) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            xmin,
            xmax,
            ymin,
            ymax,
            data,
        }
    }
}

impl<C: serde::Serialize> BinaryDxf<C> {
    /// Serialize this object to a writer.
    pub fn to_writer<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        crate::util::write_object(writer, self)
    }
}
impl<C: serde::de::DeserializeOwned> BinaryDxf<C> {
    /// Read this object from a reader. Returns an error if the version does not match.
    pub fn from_reader<R: std::io::Read>(reader: &mut R) -> anyhow::Result<Self> {
        let object: Self = crate::util::read_object(reader)?;
        anyhow::ensure!(
            object.version == env!("CARGO_PKG_VERSION"),
            "Binary DXF file was created with another version, please remove and recreate"
        );
        Ok(object)
    }
}

impl<C: ClassificationToLayer> BinaryDxf<C> {
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

                    for &(x, y) in polyline {
                        write!(
                            writer,
                            "VERTEX\r\n  8\r\n{layer}\r\n 10\r\n{x}\r\n 20\r\n{y}\r\n  0\r\n",
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
pub enum ContourClassification {
    Contour,
}
impl ClassificationToLayer for ContourClassification {
    fn to_layer(&self) -> &str {
        match self {
            Self::Contour => "cont",
        }
    }
}

/// Classification used for cliffs

/// Classification used for contour generation
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum CliffClassification {
    Cliff2,
    Cliff3,
    Cliff4,
}
impl ClassificationToLayer for CliffClassification {
    fn to_layer(&self) -> &str {
        match self {
            Self::Cliff2 => "cliff2",
            Self::Cliff3 => "cliff3",
            Self::Cliff4 => "cliff4",
        }
    }
}
