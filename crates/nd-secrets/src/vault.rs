use std::collections::BTreeMap;
use std::path::Path;

use crate::{Result, other};

type Seal = fn(&[u8]) -> Result<Vec<u8>>;

fn load(path: &Path) -> Result<BTreeMap<String, String>> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(other),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(other(e)),
    }
}

fn save(path: &Path, map: &BTreeMap<String, String>) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(map).map_err(other)?).map_err(other)?;
    std::fs::rename(&tmp, path).map_err(other)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn unhex(s: &str) -> Result<Vec<u8>> {
    (0..s.len())
        .step_by(2)
        .map(|i| {
            s.get(i..i + 2)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| other("damaged secrets file"))
        })
        .collect()
}

pub(crate) fn get(path: &Path, key: &str, open: Seal) -> Result<Option<String>> {
    let Some(sealed) = load(path)?.remove(key) else {
        return Ok(None);
    };
    String::from_utf8(open(&unhex(&sealed)?)?)
        .map(Some)
        .map_err(other)
}

pub(crate) fn set(path: &Path, key: &str, value: &str, seal: Seal) -> Result<()> {
    let mut map = load(path)?;
    map.insert(key.to_owned(), hex(&seal(value.as_bytes())?));
    save(path, &map)
}

pub(crate) fn delete(path: &Path, key: &str) -> Result<()> {
    let mut map = load(path)?;
    if map.remove(key).is_some() {
        save(path, &map)?;
    }
    Ok(())
}
