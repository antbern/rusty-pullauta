use crate::io::bytes::FromToBytes;
use std::{
    io::{Read, Seek, Write},
    time::Instant,
};

use log::debug;

/// The magic number that identifies a valid XYZ binary file.
const XYZ_MAGIC: &[u8] = b"XYZB";

/// A single record of an observed laser data point needed by the algorithms.
#[derive(Debug, Clone, PartialEq)]
pub struct XyzRecord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub classification: u8,
    pub number_of_returns: u8,
    pub return_number: u8,
}

impl FromToBytes for XyzRecord {
    fn from_bytes<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let x = f64::from_bytes(reader)?;
        let y = f64::from_bytes(reader)?;
        let z = f64::from_bytes(reader)?;

        let mut buff = [0; 3];
        reader.read_exact(&mut buff)?;
        let classification = buff[0];
        let number_of_returns = buff[1];
        let return_number = buff[2];
        Ok(Self {
            x,
            y,
            z,
            classification,
            number_of_returns,
            return_number,
        })
    }

    fn to_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // write the x, y, z coordinates
        self.x.to_bytes(writer)?;
        self.y.to_bytes(writer)?;
        self.z.to_bytes(writer)?;

        // write the classification, number of returns, return number, and intensity
        writer.write_all(&[
            self.classification,
            self.number_of_returns,
            self.return_number,
        ])
    }
}

pub struct XyzInternalWriter<W: Write + Seek> {
    inner: Option<W>,
    header: Header,
    // for stats
    start: Option<Instant>,
}

/// File header containing information about the data, such as number of records and the min/max in
/// each dimension.
#[derive(Debug, PartialEq)]
pub struct Header {
    n_records: u64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    min_z: f64,
    max_z: f64,
}

impl FromToBytes for Header {
    fn from_bytes<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let n_records = u64::from_bytes(reader)?;
        let min_x = f64::from_bytes(reader)?;
        let max_x = f64::from_bytes(reader)?;
        let min_y = f64::from_bytes(reader)?;
        let max_y = f64::from_bytes(reader)?;
        let min_z = f64::from_bytes(reader)?;
        let max_z = f64::from_bytes(reader)?;
        Ok(Self {
            n_records,
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
        })
    }

    fn to_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.n_records.to_bytes(writer)?;
        self.min_x.to_bytes(writer)?;
        self.max_x.to_bytes(writer)?;
        self.min_y.to_bytes(writer)?;
        self.max_y.to_bytes(writer)?;
        self.min_z.to_bytes(writer)?;
        self.max_z.to_bytes(writer)
    }
}

impl Header {
    /// Creates a new header with all values set to the extremes.
    fn new() -> Self {
        Self {
            n_records: 0,
            min_x: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            min_y: f64::INFINITY,
            max_y: f64::NEG_INFINITY,
            min_z: f64::INFINITY,
            max_z: f64::NEG_INFINITY,
        }
    }

    /// Updates the header with the values from the given record.
    fn update(&mut self, record: &XyzRecord) {
        self.n_records += 1;
        self.min_x = self.min_x.min(record.x);
        self.max_x = self.max_x.max(record.x);
        self.min_y = self.min_y.min(record.y);
        self.max_y = self.max_y.max(record.y);
        self.min_z = self.min_z.min(record.z);
        self.max_z = self.max_z.max(record.z);
    }
}

impl<W: Write + Seek> XyzInternalWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner: Some(inner),
            header: Header::new(),
            start: None,
        }
    }

    pub fn write_record(&mut self, record: &XyzRecord) -> std::io::Result<()> {
        let inner = self.inner.as_mut().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "writer has already been finished",
            )
        })?;

        // write the header (format + length) on the first write
        if self.header.n_records == 0 {
            self.start = Some(Instant::now());

            inner.write_all(XYZ_MAGIC)?;
            // Write the header to reserve space for it
            self.header.to_bytes(inner)?;
        }

        record.to_bytes(inner)?;
        self.header.update(record);
        Ok(())
    }

    pub fn finish(&mut self) -> std::io::Result<W> {
        let mut inner = self.inner.take().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "writer has already been finished",
            )
        })?;

        // seek to the beginning of the file and write the number of records
        inner.seek(std::io::SeekFrom::Start(XYZ_MAGIC.len() as u64))?;
        self.header.to_bytes(&mut inner)?;

        // log statistics about the written records
        if let Some(start) = self.start {
            let elapsed = start.elapsed();
            debug!(
                "Wrote {} records in {:.2?} ({:.2?}/record)",
                self.header.n_records,
                elapsed,
                elapsed / self.header.n_records as u32,
            );
        }
        Ok(inner)
    }
}

impl<W: Write + Seek> Drop for XyzInternalWriter<W> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            self.finish().expect("failed to finish writer in Drop");
        }
    }
}

pub struct XyzInternalReader<R: Read> {
    inner: R,
    header: Header,
    records_read: u64,
    // for stats
    start: Option<Instant>,
}

impl<R: Read> XyzInternalReader<R> {
    pub fn new(mut inner: R) -> std::io::Result<Self> {
        // read and check the magic number
        let mut buff = [0; XYZ_MAGIC.len()];
        inner.read_exact(&mut buff)?;
        if buff != XYZ_MAGIC {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid magic number",
            ));
        }

        // read the header containing the metadata
        let header = Header::from_bytes(&mut inner)?;
        Ok(Self {
            inner,
            header,
            records_read: 0,
            start: None,
        })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> std::io::Result<Option<XyzRecord>> {
        if self.records_read >= self.header.n_records {
            // log statistics about the read records
            if let Some(start) = self.start {
                let elapsed = start.elapsed();
                debug!(
                    "Read {} records in {:.2?} ({:.2?}/record)",
                    self.records_read,
                    elapsed,
                    elapsed / self.records_read as u32,
                );
            }

            return Ok(None);
        }

        if self.records_read == 0 {
            self.start = Some(Instant::now());
        }

        let record = XyzRecord::from_bytes(&mut self.inner)?;
        self.records_read += 1;
        Ok(Some(record))
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use crate::io::xyz::XyzRecord;

    use super::*;

    #[test]
    fn test_xyz_record() {
        let record = XyzRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            classification: 4,
            number_of_returns: 5,
            return_number: 6,
        };

        let mut buff = Vec::new();
        record.to_bytes(&mut buff).unwrap();
        let read_record = XyzRecord::from_bytes(&mut buff.as_slice()).unwrap();

        assert_eq!(record, read_record);
    }

    #[test]
    fn test_header() {
        let mut header = Header::new();
        header.update(&XyzRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            classification: 4,
            number_of_returns: 5,
            return_number: 6,
        });
        header.update(&XyzRecord {
            x: -12.0,
            y: -3.0,
            z: 40.0,
            classification: 5,
            number_of_returns: 6,
            return_number: 7,
        });

        assert_eq!(header.n_records, 2);
        assert_eq!(header.min_x, -12.0);
        assert_eq!(header.max_x, 1.0);
        assert_eq!(header.min_y, -3.0);
        assert_eq!(header.max_y, 2.0);
        assert_eq!(header.min_z, 3.0);
        assert_eq!(header.max_z, 40.0);

        let mut buff = Vec::new();
        header.to_bytes(&mut buff).unwrap();
        let read_header = Header::from_bytes(&mut buff.as_slice()).unwrap();

        assert_eq!(header, read_header);
    }

    #[test]
    fn test_writer_reader_many() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = XyzInternalWriter::new(cursor);

        let record = XyzRecord {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            classification: 4,
            number_of_returns: 5,
            return_number: 6,
        };

        writer.write_record(&record).unwrap();
        writer.write_record(&record).unwrap();
        writer.write_record(&record).unwrap();

        // now read the records
        let data = writer.finish().unwrap().into_inner();
        let cursor = Cursor::new(data);
        let mut reader = super::XyzInternalReader::new(cursor).unwrap();
        assert_eq!(reader.next().unwrap().unwrap(), record);
        assert_eq!(reader.next().unwrap().unwrap(), record);
        assert_eq!(reader.next().unwrap().unwrap(), record);
        assert_eq!(reader.next().unwrap(), None);
    }
}
