use std::convert::TryFrom;
use std::io::Write;
use std::ops::Drop;
use std::str::FromStr;
extern crate deflate;
use deflate::deflate_bytes_conf;
use deflate::Compression;
mod crc32;
use crc32::CRC32;
mod time;
use time::DateTime;
mod error;
pub use error::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Eq, PartialEq, Clone, Copy)]
pub enum Level {
    Raw,
    Low,
    Default,
    High,
}

impl Level {
    fn method(&self) -> u16 {
        if *self == Level::Raw {
            0
        } else {
            8
        }
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
        ZipEntry {
            method,
            timestamp: DateTime::now().dos_time(),
            checksum: uncompressed_content.iter().crc32(),
            compressed_size: compressed_content.len() as u32,
            uncompressed_size: uncompressed_content.len() as u32,
            offset: offset,
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

pub struct ZipArchive<'a, T: Write + 'a> {
    state: ZipState,
    output: &'a mut T,
    entries: Vec<ZipEntry>,
    offset: u32,
}

impl<'a, T: Write + 'a> ZipArchive<'a, T> {
    pub fn new(output: &'a mut T) -> ZipArchive<'a, T> {
        ZipArchive {
            state: ZipState::Breathe,
            output,
            entries: Vec::<ZipEntry>::new(),
            offset: 0,
        }
    }

    fn pk0304(output: &mut T, entry: &ZipEntry) -> Result<u32> {
        let mut write_size = output.write(&0x04034b50u32.to_le_bytes())?;
        write_size += output.write(&20u16.to_le_bytes())?;
        write_size += output.write(&2048u16.to_le_bytes())?;
        write_size += output.write(&entry.method.to_le_bytes())?;
        write_size += output.write(&entry.timestamp.to_le_bytes())?;
        write_size += output.write(&entry.checksum.to_le_bytes())?;
        write_size += output.write(&entry.compressed_size.to_le_bytes())?;
        write_size += output.write(&entry.uncompressed_size.to_le_bytes())?;
        write_size += output.write(&u16::try_from(entry.filename.len())?.to_le_bytes())?;
        write_size += output.write(&0u16.to_le_bytes())?;
        write_size += output.write(entry.filename.as_bytes())?;
        Ok(u32::try_from(write_size)?)
    }

    pub fn add_entry(mut self, name: &str, content: &[u8], level: Level) -> Result<Self> {
        self.state = ZipState::Processing;
        if let Some(compression) = level.compression() {
            let compressed_body = deflate_bytes_conf(content, compression);
            let entry = ZipEntry::new(name, content, &compressed_body, level.method(), self.offset);
            self.offset += Self::pk0304(self.output, &entry)? as u32;
            self.offset += self.output.write(compressed_body.as_slice())? as u32;
            self.entries.push(entry);
        } else {
            let entry = ZipEntry::new(name, content, &content, level.method(), self.offset);
            self.offset += Self::pk0304(self.output, &entry)? as u32;
            self.offset += self.output.write(content)? as u32;
            self.entries.push(entry);
        }
        self.state = ZipState::Breathe;
        Ok(self)
    }

    fn pk0102(output: &mut T, entry: &ZipEntry) -> Result<u32> {
        let mut write_size = output.write(&0x02014b50u32.to_le_bytes())?;
        write_size += output.write(&20u16.to_le_bytes())?;
        write_size += output.write(&20u16.to_le_bytes())?;
        write_size += output.write(&2048u16.to_le_bytes())?;
        write_size += output.write(&entry.method.to_le_bytes())?;
        write_size += output.write(&entry.timestamp.to_le_bytes())?;
        write_size += output.write(&entry.checksum.to_le_bytes())?;
        write_size += output.write(&entry.compressed_size.to_le_bytes())?;
        write_size += output.write(&entry.uncompressed_size.to_le_bytes())?;
        write_size += output.write(&(u32::try_from(entry.filename.len())?).to_le_bytes())?;
        write_size += output.write(&0u16.to_le_bytes())?;
        write_size += output.write(&0u16.to_le_bytes())?;
        write_size += output.write(&0u16.to_le_bytes())?;
        write_size += output.write(&0u16.to_le_bytes())?;
        write_size += output.write(&0u16.to_le_bytes())?;
        write_size += output.write(&entry.offset.to_le_bytes())?;
        write_size += output.write(entry.filename.as_bytes())?;
        Ok(u32::try_from(write_size)?)
    }

    pub fn flush(mut self) -> Result<()> {
        self.state = ZipState::Processing;
        let entries = std::mem::take(&mut self.entries);
        let top_of_central_directory = self.offset;
        for entry in entries.iter() {
            self.offset += Self::pk0102(&mut self.output, entry)?;
        }
        let size_of_the_central_directory = self.offset - top_of_central_directory;
        self.output.write(&0x06054b50u32.to_le_bytes())?;
        self.output.write(&0u32.to_le_bytes())?;
        self.output.write(&(entries.len() as u16).to_le_bytes())?;
        self.output.write(&(entries.len() as u16).to_le_bytes())?;
        self.output
            .write(&size_of_the_central_directory.to_le_bytes())?;
        self.output.write(&top_of_central_directory.to_le_bytes())?;
        self.output.write(&0u16.to_le_bytes())?;
        self.state = ZipState::Finished;
        Ok(())
    }
}

impl<'a, T: Write + 'a> Drop for ZipArchive<'a, T> {
    fn drop(&mut self) {
        if self.state == ZipState::Breathe {
            self.state = ZipState::Processing;
            let entries = std::mem::take(&mut self.entries);
            let top_of_central_directory = self.offset;
            for entry in entries.iter() {
                self.offset += Self::pk0102(&mut self.output, entry).unwrap();
            }
            let size_of_the_central_directory = self.offset - top_of_central_directory;
            self.output.write(&0x06054b50u32.to_le_bytes()).unwrap();
            self.output.write(&0u32.to_le_bytes()).unwrap();
            self.output
                .write(&(entries.len() as u16).to_le_bytes())
                .unwrap();
            self.output
                .write(&(entries.len() as u16).to_le_bytes())
                .unwrap();
            self.output
                .write(&size_of_the_central_directory.to_le_bytes())
                .unwrap();
            self.output
                .write(&top_of_central_directory.to_le_bytes())
                .unwrap();
            self.output.write(&0u16.to_le_bytes()).unwrap();
            self.state = ZipState::Finished;
        }
    }
}
