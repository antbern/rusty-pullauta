use crate::io::bytes::FromToBytes;
use std::{
    io::{Read, Seek, Write},
    time::Instant,
};

use log::debug;

/// The magic number that identifies a valid XYZ binary file.
const XYZ_MAGIC: &[u8] = b"XYZB";

/// A single record of an observed laser data point needed by the algorithms.
// #[derive(Debug, Default, Clone, PartialEq, zerocopy::FromBytes, zerocopy::IntoBytes)]
// #[repr(C)]
#[derive(Debug, Default, Clone, PartialEq)]
#[repr(C)]
pub struct XyzRecord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub classification: u8,
    pub number_of_returns: u8,
    pub return_number: u8,
}

impl XyzRecord {
    // Cannot use std::mem::size_of::<Self>() here because that includes the padding
    // bytes at the end, instead we can use the offset of the last field plus its size.
    // The unit test test_xyz_record_no_inner_padding ensures that there is only padding at the end
    // of the struct and that this size is the total sum of the sizes of all fields.
    const CONTIGOUS_BYTE_SIZE: usize =
        std::mem::offset_of!(XyzRecord, return_number) + size_of::<u8>();

    fn as_slice(&self) -> &[u8] {
        // SAFETY: this is safe since alignment is guaranteed by the `repr(C)` attribute, and
        // there is no padding between the fields. The number of bytes is checked to be the exact
        // length before the padding bytes start.
        unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8, Self::CONTIGOUS_BYTE_SIZE)
        }
    }
    fn as_slice_mut(&mut self) -> &mut [u8] {
        // SAFETY: this is safe since alignment is guaranteed by the `repr(C)` attribute, and
        // there is no padding between the fields. The number of bytes is checked to be the exact
        // length before the padding bytes start.
        unsafe {
            std::slice::from_raw_parts_mut(self as *mut Self as *mut u8, Self::CONTIGOUS_BYTE_SIZE)
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::io::xyz::XyzRecord;

    #[test]
    fn test_xyz_record_no_inner_padding() {
        let record = XyzRecord::default();

        let mut size = 0;
        assert_eq!(size, std::mem::offset_of!(XyzRecord, x));
        size += size_of_val(&record.x);

        assert_eq!(size, std::mem::offset_of!(XyzRecord, y));
        size += size_of_val(&record.y);

        assert_eq!(size, std::mem::offset_of!(XyzRecord, z));
        size += size_of_val(&record.z);

        assert_eq!(size, std::mem::offset_of!(XyzRecord, classification));
        size += size_of_val(&record.classification);

        assert_eq!(size, std::mem::offset_of!(XyzRecord, number_of_returns));
        size += size_of_val(&record.number_of_returns);

        assert_eq!(size, std::mem::offset_of!(XyzRecord, return_number));
        size += size_of_val(&record.return_number);

        assert_eq!(size, XyzRecord::CONTIGOUS_BYTE_SIZE);

        assert!(align_of::<XyzRecord>() > align_of::<u8>());
        assert_eq!(align_of::<&XyzRecord>(), align_of::<&[u8]>());
    }
}

impl FromToBytes for XyzRecord {
    fn from_bytes<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut record = Self::default();
        let buffer = record.as_slice_mut();
        reader.read_exact(buffer)?;

        Ok(record)
    }

    fn to_bytes<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.as_slice())?;
        Ok(())
    }
}

pub struct XyzInternalWriter<W: Write + Seek> {
    inner: Option<W>,
    records_written: u64,
    // for stats
    start: Option<Instant>,
}

impl<W: Write + Seek> XyzInternalWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner: Some(inner),
            records_written: 0,
            start: None,
        }
    }

    pub fn write_record(&mut self, record: &XyzRecord) -> std::io::Result<()> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| std::io::Error::other("writer has already been finished"))?;

        // write the header (format + length) on the first write
        if self.records_written == 0 {
            self.start = Some(Instant::now());

            inner.write_all(XYZ_MAGIC)?;
            // Write the temporary number of records as all FF
            u64::MAX.to_bytes(inner)?;
        }

        // do the magic

        let buffer = record.as_slice();

        inner.write_all(buffer)?;
        self.records_written += 1;
        Ok(())
    }

    pub fn finish(&mut self) -> std::io::Result<W> {
        let mut inner = self
            .inner
            .take()
            .ok_or_else(|| std::io::Error::other("writer has already been finished"))?;

        // seek to the beginning of the file and write the number of records
        inner.seek(std::io::SeekFrom::Start(XYZ_MAGIC.len() as u64))?;
        self.records_written.to_bytes(&mut inner)?;

        // log statistics about the written records
        if let Some(start) = self.start {
            let elapsed = start.elapsed();
            debug!(
                "Wrote {} records in {:.2?} ({:.2?}/record)",
                self.records_written,
                elapsed,
                elapsed / self.records_written as u32,
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
    n_records: u64,
    records_read: u64,
    // for stats
    start: Option<Instant>,
    record: XyzRecord,
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

        // read the number of records, defined by the first u64
        let n_records = u64::from_bytes(&mut inner)?;
        Ok(Self {
            inner,
            n_records,
            records_read: 0,
            start: None,
            record: Default::default(),
        })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> std::io::Result<Option<&XyzRecord>> {
        if self.records_read >= self.n_records {
            // TODO: log statistics about the read records
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

        // now do the magic
        let buffer = self.record.as_slice_mut();
        self.inner.read_exact(buffer)?;

        // let record = XyzRecord::from_bytes(&mut self.inner)?;
        self.records_read += 1;
        Ok(Some(&self.record))
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
        assert_eq!(reader.next().unwrap().unwrap(), &record);
        assert_eq!(reader.next().unwrap().unwrap(), &record);
        assert_eq!(reader.next().unwrap().unwrap(), &record);
        assert_eq!(reader.next().unwrap(), None);
    }
}
