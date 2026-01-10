//! This crate is to generate zip archive files.
//!
//! # ZIP generating steps
//!
//! 1. Create [`ZipArchive`] structure by [`new`](ZipArchive::new) method
//! 2. Add zip entry by [`add_entry`](ZipArchive::add_entry) method
//! 3. Write ending data by [`flush`](ZipArchive::flush) method.
//!
//! # Note
//!
//! - If the return value of a method is an error, the output data is incomplete.
//! - If you do not call `flush` method, [`drop`](ZipArchive::drop) write ending data.
//! - Failure of writing in `drop` will cause panic.
//!
//! # Example
//!
//! ```rust
//! use std::fs::File;
//! use zip_builder::Level;
//! use zip_builder::Result;
//! use zip_builder::ZipArchive;
//!
//! fn main() -> Result<()> {
//!     let mut file = File::create("foo.zip").unwrap();
//!     let zip_builder = ZipArchive::new(&mut file)
//!         .add_entry("file1.txt", b"content", Level::Low)?
//!         .add_entry("file2.txt", b"content", Level::Raw)?
//!         .flush();
//!
//!     Ok(())
//! }
//! ```

use std::convert::TryFrom;
use std::io::Write;
use std::ops::Drop;
use std::str::FromStr;
extern crate deflate;
use deflate::Compression;
use deflate::deflate_bytes_conf;
mod crc32;
use crc32::CRC32;
mod time;
use time::DateTime;
mod error;
pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Represents complression level.
#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Level {
    /// Not compress. Store raw data.
    Raw,
    /// Fast compress.
    Low,
    /// Normal compress.
    Default,
    /// Strong compress. Slowly.
    High,
}

impl Level {
    fn method(&self) -> u16 {
        if *self == Level::Raw { 0 } else { 8 }
    }
    fn compression(&self) -> Option<Compression> {
        match self {
            Level::Raw => None,
            Level::Low => Some(Compression::Fast),
            Level::Default => Some(Compression::Default),
            Level::High => Some(Compression::Best),
        }
    }
}

struct ZipEntry {
    method: u16,
    timestamp: u32,
    checksum: u32,
    compressed_size: u32,
    uncompressed_size: u32,
    offset: u32,
    filename: String,
}

impl ZipEntry {
    fn new(
        filename: &str,
        uncompressed_content: &[u8],
        compressed_content: &[u8],
        method: u16,
        offset: u32,
    ) -> ZipEntry {
        let mut hasher = CRC32::default();
        hasher.write(uncompressed_content);
        ZipEntry {
            method,
            timestamp: DateTime::now().dos_time(),
            checksum: hasher.finish(),
            compressed_size: compressed_content.len() as u32,
            uncompressed_size: uncompressed_content.len() as u32,
            offset,
            filename: String::from_str(filename).unwrap(),
        }
    }
}

#[derive(Eq, PartialEq)]
enum ZipState {
    Processing,
    Breathe,
    Finished,
}

/// The main struct you will need to use in this library.
pub struct ZipArchive<'a, T: Write + 'a> {
    state: ZipState,
    output: &'a mut T,
    entries: Vec<ZipEntry>,
    offset: u32,
}

impl<'a, T: Write + 'a> ZipArchive<'a, T> {
    /// Create a new [`ZipArchive`] structure.
    pub fn new(output: &'a mut T) -> ZipArchive<'a, T> {
        ZipArchive {
            state: ZipState::Breathe,
            output,
            entries: Vec::<ZipEntry>::new(),
            offset: 0,
        }
    }

    fn pk0304(output: &mut T, entry: &ZipEntry) -> Result<u32> {
        output.write_all(&0x04034b50u32.to_le_bytes())?;
        output.write_all(&20u16.to_le_bytes())?;
        output.write_all(&2048u16.to_le_bytes())?;
        output.write_all(&entry.method.to_le_bytes())?;
        output.write_all(&entry.timestamp.to_le_bytes())?;
        output.write_all(&entry.checksum.to_le_bytes())?;
        output.write_all(&entry.compressed_size.to_le_bytes())?;
        output.write_all(&entry.uncompressed_size.to_le_bytes())?;
        output.write_all(&u16::try_from(entry.filename.len())?.to_le_bytes())?;
        output.write_all(&0u16.to_le_bytes())?;
        output.write_all(entry.filename.as_bytes())?;
        Ok(u32::try_from(30 + entry.filename.len())?)
    }

    /// Add a entry to the zip.
    ///
    /// Level means compression level.
    pub fn add_entry(&mut self, name: &str, content: &[u8], level: Level) -> Result<&mut Self> {
        if self.state != ZipState::Breathe {
            return Err(Error::AttemptWriteClosedArchive);
        }
        self.state = ZipState::Processing;
        if let Some(compression) = level.compression() {
            let compressed_body = deflate_bytes_conf(content, compression);
            let entry = ZipEntry::new(name, content, &compressed_body, level.method(), self.offset);
            self.offset += Self::pk0304(self.output, &entry)? as u32;
            self.output.write_all(compressed_body.as_slice())?;
            self.offset += compressed_body.len() as u32;
            self.entries.push(entry);
        } else {
            let entry = ZipEntry::new(name, content, content, level.method(), self.offset);
            self.offset += Self::pk0304(self.output, &entry)?;
            self.output.write_all(content)?;
            self.offset += content.len() as u32;
            self.entries.push(entry);
        }
        self.state = ZipState::Breathe;
        Ok(self)
    }

    fn pk0102(output: &mut T, entry: &ZipEntry) -> Result<u32> {
        output.write_all(&0x02014b50u32.to_le_bytes())?;
        output.write_all(&20u16.to_le_bytes())?;
        output.write_all(&20u16.to_le_bytes())?;
        output.write_all(&2048u16.to_le_bytes())?;
        output.write_all(&entry.method.to_le_bytes())?;
        output.write_all(&entry.timestamp.to_le_bytes())?;
        output.write_all(&entry.checksum.to_le_bytes())?;
        output.write_all(&entry.compressed_size.to_le_bytes())?;
        output.write_all(&entry.uncompressed_size.to_le_bytes())?;
        output.write_all(&(u32::try_from(entry.filename.len())?).to_le_bytes())?;
        output.write_all(&0u16.to_le_bytes())?;
        output.write_all(&0u16.to_le_bytes())?;
        output.write_all(&0u16.to_le_bytes())?;
        output.write_all(&0u16.to_le_bytes())?;
        output.write_all(&0u16.to_le_bytes())?;
        output.write_all(&entry.offset.to_le_bytes())?;
        output.write_all(entry.filename.as_bytes())?;
        Ok(u32::try_from(46 + entry.filename.len())?)
    }

    /// Write ending data.
    ///
    /// Specifically, central directory header (PK0102) and end of central directory record (PK0506).
    pub fn flush(&mut self) -> Result<()> {
        if self.state != ZipState::Breathe {
            return Err(Error::AttemptWriteClosedArchive);
        }
        self.state = ZipState::Processing;
        let entries = std::mem::take(&mut self.entries);
        let top_of_central_directory = self.offset;
        for entry in entries.iter() {
            self.offset += Self::pk0102(self.output, entry)?;
        }
        let size_of_the_central_directory = self.offset - top_of_central_directory;
        self.output.write_all(&0x06054b50u32.to_le_bytes())?;
        self.output.write_all(&0u32.to_le_bytes())?;
        self.output
            .write_all(&(entries.len() as u16).to_le_bytes())?;
        self.output
            .write_all(&(entries.len() as u16).to_le_bytes())?;
        self.output
            .write_all(&size_of_the_central_directory.to_le_bytes())?;
        self.output
            .write_all(&top_of_central_directory.to_le_bytes())?;
        self.output.write_all(&0u16.to_le_bytes())?;
        self.state = ZipState::Finished;
        Ok(())
    }
}

impl<'a, T: Write + 'a> Drop for ZipArchive<'a, T> {
    /// If flush method has be not called, this method write ending data.
    /// But failing to write causes a panic.
    /// It is recommended to always call [`flush`](ZipArchive::flush) explicitly.
    fn drop(&mut self) {
        if self.state == ZipState::Breathe {
            self.flush().unwrap();
        }
    }
}
