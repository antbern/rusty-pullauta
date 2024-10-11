use std::{
    fmt::Debug,
    fs::File,
    io::{self, BufRead},
    path::Path,
    sync::{LazyLock, Mutex},
    time::Instant,
};

use log::debug;

use crate::config::Config;

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

/// Iterates over the lines in a file and calls the callback with a &str reference to each line.
/// This function does not allocate new strings for each line, as opposed to using
/// [`io::BufReader::lines()`] as in [`read_lines`].
pub fn read_lines_no_alloc<P>(filename: P, mut line_callback: impl FnMut(&str)) -> io::Result<()>
where
    P: AsRef<Path> + Debug,
{
    debug!("Reading lines from {filename:?}");
    let start = Instant::now();

    let file = File::open(filename)?;
    let mut reader = io::BufReader::new(file);

    let mut line_buffer = String::new();
    let mut line_count: u32 = 0;
    let mut byte_count: usize = 0;
    loop {
        let bytes_read = reader.read_line(&mut line_buffer)?;

        if bytes_read == 0 {
            break;
        }

        line_count += 1;
        byte_count += bytes_read;

        // the read line contains the newline delimiter, so we need to trim it off
        let line = line_buffer.trim_end();
        line_callback(line);
        line_buffer.clear();
    }

    let elapsed = start.elapsed();
    debug!(
        "Read {} lines in {:.2?} ({:.2?}/line), total {} bytes ({:.2} bytes/second, {:?}/byte, {:.2} bytes/line)",
        line_count,
        elapsed,
        elapsed / line_count,
        byte_count,
        byte_count as f64 / elapsed.as_secs_f64(),
        elapsed / byte_count as u32,
        byte_count as f64 / line_count as f64,
    );

    Ok(())
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

struct CachedPoints {
    locations: Box<[PointLocation]>,
    metadata: Box<[PointMetadata]>,
}

/// Global shared storage
static POINT_CACHE: LazyLock<Mutex<std::collections::HashMap<String, CachedPoints>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

/// Reads the input file and caches the content in memory if that is enabled in the config
pub fn read_xyztemp_input_file(
    filename: &Path,
    config: &Config,
    mut callback: impl FnMut(&PointLocation, &PointMetadata),
) -> Result<(), Box<dyn std::error::Error>> {
    let should_cache = config.cache_input_points;

    if should_cache {
        if let Some(cached_points) = POINT_CACHE
            .lock()
            .expect("should not be poisoned")
            .get(&filename.display().to_string())
        {
            debug!("Using cached points for {}", filename.display());
            for (l, m) in cached_points
                .locations
                .iter()
                .zip(cached_points.metadata.iter())
            {
                callback(l, m);
            }
            return Ok(());
        }
    }

    debug!("Reading points from {}", filename.display());
    let (mut point_locations, mut point_metadata) = (Vec::new(), Vec::new());
    read_lines_no_alloc(filename, |line| {
        let mut parts = line.trim().split(' ');

        let x = parts.next().unwrap().parse::<f64>().unwrap();
        let y = parts.next().unwrap().parse::<f64>().unwrap();
        let z = parts.next().unwrap().parse::<f64>().unwrap();
        let classification = parts.next().unwrap().parse::<u8>().unwrap();
        let number_of_returns = parts.next().unwrap().parse::<u8>().unwrap();
        let return_number = parts.next().unwrap().parse::<u8>().unwrap();
        let intensity = parts.next().unwrap().parse::<u16>().unwrap();

        let l = PointLocation { x, y, z };
        let m = PointMetadata {
            classification,
            number_of_returns,
            return_number,
            intensity,
        };

        callback(&l, &m);
        if should_cache {
            point_locations.push(l);
            point_metadata.push(m);
        }
    })?;

    if should_cache {
        POINT_CACHE.lock().expect("should not be poisoned").insert(
            filename.display().to_string(),
            CachedPoints {
                locations: point_locations.into_boxed_slice(),
                metadata: point_metadata.into_boxed_slice(),
            },
        );
        debug!("Cached points for {}", filename.display());
    }

    Ok(())
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
