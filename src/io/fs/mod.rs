use std::{
    io::{self, BufRead, Seek, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;

pub mod local;
pub mod memory;

/// Trait for file system operations.
pub trait FileSystem: std::fmt::Debug {
    /// Create a new directory.
    fn create_dir_all(&self, path: impl AsRef<Path>) -> Result<(), io::Error>;

    /// List the contents of a directory.
    fn list(&self, path: impl AsRef<Path>) -> Result<Vec<PathBuf>, io::Error>;

    /// Check if a file exists.
    fn exists(&self, path: impl AsRef<Path>) -> bool;

    /// Open a file for reading. This is always Buffered.
    fn open(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<impl BufRead + Seek + Send + 'static, io::Error>;

    /// Open a file for writing. This is always Buffered.
    fn create(&self, path: impl AsRef<Path>) -> Result<impl Write + Seek, io::Error>;

    /// Read a file into a String.
    fn read_to_string(&self, path: impl AsRef<Path>) -> Result<String, io::Error>;

    /// Remove a file.
    fn remove_file(&self, path: impl AsRef<Path>) -> Result<(), io::Error>;

    /// Remove a dir.
    fn remove_dir_all(&self, path: impl AsRef<Path>) -> Result<(), io::Error>;

    /// Get the size of a file in bytes.
    fn file_size(&self, path: impl AsRef<Path>) -> Result<u64, io::Error>;

    /// Copy a file.
    fn copy(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<(), io::Error>;

    /// Read a serialized object from a file. Returns a reference-counted object handle.
    fn read_object<O: serde::de::DeserializeOwned + Send + Sync + std::any::Any + 'static>(
        &self,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<ReadObject<O>> {
        // default implementation just reads the object using bincode
        let mut reader = self.open(path).context("opening file for reading")?;
        let value: bincode::serde::Compat<O> =
            bincode::decode_from_std_read(&mut reader, bincode::config::standard())
                .context("deserializing from file")?;
        Ok(ReadObject::Owned(value.0))
    }

    /// Write an object to a file.
    fn write_object<O: serde::Serialize + Send + Sync + std::any::Any + 'static>(
        &self,
        path: impl AsRef<Path>,
        value: O,
    ) -> anyhow::Result<()> {
        // default implementation just stores the object using bincode
        let mut writer = self.create(path).context("creating file for writing")?;
        bincode::encode_into_std_write(
            bincode::serde::Compat(value),
            &mut writer,
            bincode::config::standard(),
        )
        .context("serializing to file")?;
        Ok(())
    }

    /// Read an image in PNG format.
    fn read_image_png(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<image::DynamicImage, image::error::ImageError> {
        let mut reader = image::ImageReader::new(self.open(path).expect("Could not open file"));
        reader.set_format(image::ImageFormat::Png);
        reader.decode()
    }
}

/// An Object that has been read from the file system.
pub enum ReadObject<T> {
    Owned(T),
    Shared(Arc<T>),
}

impl<T> std::ops::Deref for ReadObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            ReadObject::Owned(value) => value,
            ReadObject::Shared(value) => value,
        }
    }
}

impl<T: Clone> ReadObject<T> {
    /// Convert the object to an owned value.
    pub fn into_owned(self) -> T {
        match self {
            ReadObject::Owned(value) => value,
            ReadObject::Shared(value) => (*value).clone(),
        }
    }
}
