
use std::fs::File;
use std::io::{BufWriter, Write};

use serde::Serialize;
use serde_json;

pub fn write_json_file<T: Serialize>(fname: &str, what: &T) -> std::io::Result<()> {
    let file = File::create(fname)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, &what)?;
    writer.flush()?;
    Ok(())
}