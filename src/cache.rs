use std::{
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use log::{trace, warn};
use rustc_hash::FxHasher;

pub struct CachedComputation {
    dependencies_hash: u64,
    input_file: PathBuf,
    cache_file: PathBuf,
}

pub struct ComputationGuard {
    cache_file: PathBuf,
    new_cache_tag: Option<String>,
}

impl CachedComputation {
    /// Creates a new [`CachedComputation`] instance that will read from the given input file and write to the given cache file.
    /// If the cache file exists and is newer than the input file, the computation will be skipped.
    /// If the environment variable `NO_CACHE` is set, the cache will be ignored.
    ///
    /// Call [`Self::needs_recompute`] to check if the computation needs to be done. If it returns `None`, the computation can be skipped.
    /// If it returns `Some`, the computation should be done and the returned [`ComputationGuard`] should be finalized after the computation is done to update the cache file.
    pub fn new<F>(input_file: &Path, cache_file: &Path, dependencies: F) -> Self
    where
        F: FnOnce(&mut DefaultHasher),
    {
        // get the hash of the input dependencies
        let mut hasher = DefaultHasher::new();
        dependencies(&mut hasher);
        let dependencies_hash = hasher.finish();

        Self {
            dependencies_hash,
            input_file: input_file.into(),
            cache_file: cache_file.into(),
        }
    }

    /// Checks if the computation needs to be done.
    pub fn needs_recompute(&mut self) -> Option<ComputationGuard> {
        if std::env::var("NO_CACHE").is_ok() {
            warn!("NO_CACHE is set, ignoring cache");
            return Some(ComputationGuard {
                cache_file: self.cache_file.clone(),
                new_cache_tag: None,
            });
        }
        match self.needs_recompute_fallible() {
            Ok(needs_recompute) => needs_recompute,
            Err(e) => {
                warn!("Error checking cache: {:?}", e);
                Some(ComputationGuard {
                    cache_file: self.cache_file.clone(),
                    new_cache_tag: None,
                })
            }
        }
    }

    fn needs_recompute_fallible(
        &mut self,
    ) -> Result<Option<ComputationGuard>, Box<dyn std::error::Error>> {
        let modified = std::fs::metadata(&self.input_file).and_then(|m| m.modified())?;
        let file_content_hash = file_content_hash(&self.input_file)?;

        // compute the grand hash
        let mut hasher = DefaultHasher::new();
        env!("CARGO_PKG_VERSION").hash(&mut hasher); // to make sure the cache is invalidated when the version changes
        self.dependencies_hash.hash(&mut hasher);
        self.input_file.hash(&mut hasher);
        modified.hash(&mut hasher);
        file_content_hash.hash(&mut hasher);
        let expected_tag = hasher.finish().to_string();

        // if the cache file doesn't exist, we need to recompute either way
        let existing_tag = if self.cache_file.exists() {
            std::fs::read_to_string(&self.cache_file)?
        } else {
            trace!("Cache file '{}' does not exist", self.cache_file.display());
            return Ok(Some(ComputationGuard {
                cache_file: self.cache_file.clone(),
                new_cache_tag: Some(expected_tag),
            }));
        };

        let needs_recompute = existing_tag != expected_tag;
        trace!(
            "existing_tag: {:?}, expected_tag: {:?}, needs_recompute: {}",
            existing_tag,
            expected_tag,
            needs_recompute
        );

        if !needs_recompute {
            return Ok(None);
        }

        Ok(Some(ComputationGuard {
            cache_file: self.cache_file.clone(),
            new_cache_tag: Some(expected_tag),
        }))
    }
}

impl ComputationGuard {
    /// Call this method to signal that the computation is done and that the cache file should be written.
    pub fn finalize(self) {
        // so we can write the cache file
        if let Some(new_cache_tag) = self.new_cache_tag {
            if let Err(e) = std::fs::write(&self.cache_file, new_cache_tag) {
                warn!("Error writing cache file {:?}: {:?}", self.cache_file, e);
            }
        }
    }
}

/// Computes the hash of the content of the given file.
pub fn file_content_hash(file: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let start = std::time::Instant::now();

    let mut hasher = FxHasher::default();
    let mut reader = BufReader::new(File::open(file)?);

    // Read the file in 4KB chunks and feed them to the hasher
    let mut buffer = [0; 4096];
    let mut total_bytes_read = 0;
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        total_bytes_read += bytes_read;
        if bytes_read == 0 {
            break;
        }
        hasher.write(&buffer[..bytes_read])
    }
    let hash = hasher.finish();

    let elapsed = start.elapsed();
    trace!(
        "Computed hash of '{}' with size {} bytes in {:.2?} ({:.2?} bytes/s)",
        file.display(),
        total_bytes_read,
        elapsed,
        total_bytes_read as f64 / elapsed.as_secs_f64(),
    );

    Ok(hash)
}
