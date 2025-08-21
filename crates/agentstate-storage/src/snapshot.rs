use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub created_ts: i64,
    pub objects: usize,
    pub path: String,
}

pub struct SnapshotWriter {
    out: File,
    pub path: PathBuf,
}

impl SnapshotWriter {
    pub fn create(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let out = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        Ok(Self { out, path })
    }
    pub fn write_line(&mut self, obj: &serde_json::Value) -> std::io::Result<()> {
        let s = serde_json::to_string(obj).unwrap();
        self.out.write_all(s.as_bytes())?;
        self.out.write_all(b"\n")
    }
}

pub fn read_snapshot(path: PathBuf) -> std::io::Result<Vec<serde_json::Value>> {
    let fh = File::open(path)?;
    let br = BufReader::new(fh);
    let mut out = Vec::new();
    for line in br.lines() {
        if let Ok(l) = line {
            if let Ok(v) = serde_json::from_str(&l) {
                out.push(v);
            }
        }
    }
    Ok(out)
}
