use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum WalRecord {
    Put {
        ns: String,
        id: String,
        payload: serde_json::Value,
    },
    Delete {
        ns: String,
        id: String,
    },
}

pub struct Wal {
    dir: PathBuf,
    file: File,
}

impl Wal {
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join(format!("wal-{}.log", Utc::now().timestamp()));
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self { dir, file })
    }

    pub fn append(&mut self, rec: &WalRecord) -> std::io::Result<()> {
        let line = serde_json::to_string(rec).unwrap();
        self.file.write_all(line.as_bytes())?;
        self.file.write_all(b"\n")?;
        self.file.flush()?;
        Ok(())
    }

    pub fn replay(dir: PathBuf) -> std::io::Result<Vec<WalRecord>> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            let mut files: Vec<_> = rd.filter_map(|e| e.ok()).collect();
            files.sort_by_key(|e| e.file_name());
            for f in files {
                let p = f.path();
                if p.extension().and_then(|s| s.to_str()) != Some("log") {
                    continue;
                }
                let fh = File::open(&p)?;
                let br = BufReader::new(fh);
                for line in br.lines() {
                    if let Ok(l) = line {
                        if let Ok(rec) = serde_json::from_str::<WalRecord>(&l) {
                            out.push(rec);
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}
