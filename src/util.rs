use std::{
    fs::{self, File},
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use log::debug;

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// A simple line reader that reads lines from a file without allocating. Once it reaches EOF, it
/// will log statistics about the read operation, such as the number of lines read, the total number
/// of bytes read, and the time taken to read the file.
pub struct LineReader<R: BufRead> {
    reader: R,
    buffer: String,
    // for tracking statistics
    start: Option<Instant>,
    line_count: u32,
    byte_count: usize,
}

impl<R: BufRead> LineReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: String::new(),
            start: None,
            line_count: 0,
            byte_count: 0,
        }
    }
    // complains about impl Iterator for LineReader, but that
    // is not possible since we want a "LendingIterator" that returns borrowed data each time.
    #[allow(clippy::should_implement_trait)]
    /// Read the next line from the file. Returns `None` if EOF is reached.
    pub fn next(&mut self) -> io::Result<Option<&str>> {
        // start the timer the first time the function is called
        if self.start.is_none() {
            self.start = Some(Instant::now());
        }

        self.buffer.clear();
        let bytes_read = self.reader.read_line(&mut self.buffer)?;

        if bytes_read == 0 {
            // EOF reached
            self.log_statistics();

            return Ok(None);
        }

        self.line_count += 1;
        self.byte_count += bytes_read;

        // the read line contains the newline delimiter, so we need to trim it off
        return Ok(Some(self.buffer.trim_end()));
    }

    fn log_statistics(&self) {
        if let Some(start) = self.start {
            let elapsed = start.elapsed();
            if self.line_count == 0 {
                debug!("Read 0 lines in {:.2?}", elapsed);
            } else {
                debug!(
                    "Read {} lines in {:.2?} ({:.2?}/line), total {} bytes ({:.2} bytes/second, {:?}/byte, {:.2} bytes/line)",
                    self.line_count,
                    elapsed,
                    elapsed / self.line_count,
                    self.byte_count,
                    self.byte_count as f64 / elapsed.as_secs_f64(),
                    elapsed / self.byte_count as u32,
                    self.byte_count as f64 / self.line_count as f64,
                );
            }
        }
    }
}

/// Helper struct to time operations. Keeps track of the total time taken until the object is
/// dropped, as well as timing between individual sub-sections of the operation.
/// Timing information is printed using debug level log messages.
pub struct Timing {
    name: &'static str,
    start: Instant,
    current_section: Option<TimingSection>,
}

struct TimingSection {
    name: &'static str,
    start: Instant,
}

impl Timing {
    /// Start a new timing from now.
    pub fn start_now(name: &'static str) -> Self {
        debug!("[timing: {name}] Starting timing");
        Self {
            name,
            start: Instant::now(),
            current_section: None,
        }
    }

    /// Start a new timing section. This will end any already existing sections.
    pub fn start_section(&mut self, name: &'static str) {
        let now = self.end_section().unwrap_or(Instant::now());

        debug!("[timing: {}] Entering section '{}'", self.name, name);

        self.current_section = Some(TimingSection { name, start: now })
    }

    /// Ends the currnently active section and returns its end time, or does nothing
    /// if no section is active and returns `None`.
    pub fn end_section(&mut self) -> Option<Instant> {
        if let Some(s) = self.current_section.take() {
            //
            let now = Instant::now();
            debug!(
                "[timing: {}] Leaving section '{}', which took {:.3?}",
                self.name,
                s.name,
                now - s.start
            );
            Some(now)
        } else {
            None
        }
    }
}

impl Drop for Timing {
    fn drop(&mut self) {
        self.end_section();

        debug!(
            "[timing: {}] Stopping timing. Total: {:.3?} elapsed.",
            self.name,
            self.start.elapsed()
        );
    }
}

/// A provider of file readers that can be used to read lines from a file.
///
/// Creates and generates file objects that can be read / written to. Provides special
/// functionality for reading XYZ files in an efficient way. Can provide caching of read data.
pub struct FileProvider {
    base_directory: PathBuf,
}

impl FileProvider {
    pub fn new(base_directory: &Path) -> Self {
        fs::create_dir_all(base_directory).expect("Could not create tmp folder");
        Self {
            base_directory: base_directory.to_path_buf(),
        }
    }

    /// Read the XYZ records from a file in the base directory.
    pub fn xyz(&mut self, filename: &str) -> XyzReader {
        let path = self.base_directory.join(filename);
        XyzReader {
            line_reader: LineReader::new(io::BufReader::new(
                File::open(path).expect("Could not open file"),
            )),
        }
    }

    /// Read the lines of a file in the base directory.
    pub fn lines(&mut self, filename: &str) -> LineReader<io::BufReader<File>> {
        let path = self.base_directory.join(filename);
        LineReader::new(io::BufReader::new(
            File::open(path).expect("Could not open file"),
        ))
    }

    /// Read the contents of a file into a string.
    pub fn read_to_string(&mut self, filename: &str) -> io::Result<String> {
        let path = self.base_directory.join(filename);
        std::fs::read_to_string(path)
    }

    /// Write to a file in the base directory.
    pub fn write(&mut self, filename: &str) -> impl Write {
        let path = self.base_directory.join(filename);
        io::BufWriter::new(File::create(path).expect("Could not create file"))
    }

    /// Check if a file exists in the base directory.
    pub fn exists(&self, filename: &str) -> bool {
        let path = self.base_directory.join(filename);
        path.exists()
    }

    /// Get the full path to a file in the base directory.
    pub fn path(&self, filename: &str) -> PathBuf {
        self.base_directory.join(filename)
    }

    /// Copy from an external file to the base directory.
    pub fn copy_from_outside(&self, from: &str, to: &str) -> io::Result<()> {
        fs::copy(from, self.base_directory.join(to)).map(|_| ())
    }
    /// Copy files within the base directory.
    pub fn copy(&self, from: &str, to: &str) -> io::Result<()> {
        fs::copy(self.base_directory.join(from), self.base_directory.join(to)).map(|_| ())
    }
}

pub struct PointLocation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub struct PointMetadata {
    pub classification: u8,
    pub number_of_returns: u8,
    pub return_number: u8,
    pub intensity: u16,
}

// struct CachedPoints {
//     locations: Box<[PointLocation]>,
//     metadata: Option<Box<[PointMetadata]>>,
// }

pub struct XyzReader {
    line_reader: LineReader<io::BufReader<File>>,
}

pub struct PartiallyParsedXyz<'a> {
    /// The parsed point location.
    pub p: PointLocation,

    /// The rest of the line that was not parsed, potentially contains metadata.
    the_rest: Option<&'a str>,
}

impl<'a> PartiallyParsedXyz<'a> {
    pub fn from_str(line: &'a str) -> Self {
        // only parse the first three values
        let mut parts = line.splitn(3 + 1, ' ');

        let x = parts.next().unwrap().parse().unwrap();
        let y = parts.next().unwrap().parse().unwrap();
        let z = parts.next().unwrap().parse().unwrap();
        // the last part is the rest of the line
        let the_rest = parts.next();

        Self {
            p: PointLocation { x, y, z },
            the_rest,
        }
    }
    /// Parse the metadata from the rest of the line.
    pub fn metadata(&self) -> Option<PointMetadata> {
        let mut parts = self.the_rest?.split(' ');

        let classification: u8 = parts.next()?.parse().ok()?;
        let number_of_returns: u8 = parts.next()?.parse().ok()?;
        let return_number: u8 = parts.next()?.parse().ok()?;
        let intensity: u16 = parts.next()?.parse().ok()?;

        Some(PointMetadata {
            classification,
            number_of_returns,
            return_number,
            intensity,
        })
    }

    pub fn the_rest(&self) -> &str {
        self.the_rest.unwrap_or("")
    }
}

impl XyzReader {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> io::Result<Option<PartiallyParsedXyz<'_>>> {
        let line = match self.line_reader.next() {
            Ok(Some(line)) => line,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        Ok(Some(PartiallyParsedXyz::from_str(line)))
    }
}
