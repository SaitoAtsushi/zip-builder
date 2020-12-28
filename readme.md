# zip-builder

A library to generate zip archive files.

## Usage

To use zip-builder, add this to your Cargo.toml:

```toml
[dependencies]
zip-builder = {git = "https://github.com/SaitoAtsushi/zip-builder.git" }
```

## Example

```
use std::fs::File;
use zip_builder::Level;
use zip_builder::Result;
use zip_builder::ZipArchive;

fn main() -> Result<()> {
    let mut file = File::create("foo.zip").unwrap();
    let zip_builder = ZipArchive::new(&mut file)
        .add_entry("file1.txt", b"content", Level::Low)?
        .add_entry("file2.txt", b"content", Level::Raw)?
        .flush();

    Ok(())
}
```

If it is not flushed, it will be cleaned up with `drop`.
But if fail in `drop`,  panic is caused.
