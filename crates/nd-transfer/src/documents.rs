use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::de::{Deserializer as _, SeqAccess, Visitor};
use serde_json::Value;

use crate::Result;
use crate::read::Counted;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Doc {
    pub n: u64,
    pub text: std::result::Result<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum DocLayout {
    Array,
    Lines,
    Stream,
}

pub type Seen = (u64, u64);

const STOP: &str = "nd-transfer: stop";

fn doc(n: u64, v: Value) -> Doc {
    Doc {
        n,
        text: if v.is_object() {
            Ok(v.to_string())
        } else {
            Err("not a JSON object".to_owned())
        },
    }
}

struct Out<'a> {
    sink: &'a mut dyn FnMut(Vec<Doc>, Seen) -> bool,
    buf: Vec<Doc>,
    size: usize,
    seen: Arc<AtomicU64>,
    total: u64,
    stopped: bool,
    last: u64,
}

impl Out<'_> {
    fn push(&mut self, d: Doc) -> bool {
        self.last = d.n;
        self.buf.push(d);
        if self.buf.len() >= self.size {
            return self.flush();
        }
        true
    }

    fn flush(&mut self) -> bool {
        if self.stopped {
            return false;
        }
        let batch = std::mem::take(&mut self.buf);
        let at = (
            self.seen.load(Ordering::Relaxed).min(self.total),
            self.total,
        );
        if !(self.sink)(batch, at) {
            self.stopped = true;
        }
        !self.stopped
    }
}

struct Each<'s, 'a>(&'s mut Out<'a>, u64);

impl<'de> Visitor<'de> for Each<'_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("an array of documents")
    }
    fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> std::result::Result<(), A::Error> {
        while let Some(v) = seq.next_element::<Value>()? {
            self.1 += 1;
            if !self.0.push(doc(self.1, v)) {
                return Err(serde::de::Error::custom(STOP));
            }
        }
        Ok(())
    }
}

pub fn read_documents(
    path: &Path,
    size: usize,
    sink: &mut dyn FnMut(Vec<Doc>, Seen) -> bool,
) -> Result<DocLayout> {
    let total = std::fs::metadata(path)?.len();
    let seen = Arc::new(AtomicU64::new(0));
    let mut file = BufReader::new(Counted {
        inner: File::open(path)?,
        seen: seen.clone(),
    });
    let mut out = Out {
        sink,
        buf: Vec::new(),
        size: size.max(1),
        seen,
        total,
        stopped: false,
        last: 0,
    };
    let mut skipped = 0u64;
    let first = loop {
        let buf = file.fill_buf()?;
        let Some(&b) = buf.first() else {
            out.flush();
            return Ok(DocLayout::Lines);
        };
        if buf.starts_with(b"\xef\xbb\xbf") {
            file.consume(3);
        } else if b == b'\n' {
            skipped += 1;
            file.consume(1);
        } else if b.is_ascii_whitespace() {
            file.consume(1);
        } else {
            break b;
        }
    };
    if first == b'[' {
        let mut de = serde_json::Deserializer::from_reader(file.by_ref());
        if let Err(e) = de.deserialize_seq(Each(&mut out, 0))
            && !e.to_string().starts_with(STOP)
        {
            let n = out.last + 1;
            out.push(Doc {
                n,
                text: Err(e.to_string()),
            });
        }
        out.flush();
        return Ok(DocLayout::Array);
    }
    let mut line = String::new();
    file.read_line(&mut line)?;
    if serde_json::from_str::<Value>(line.trim()).is_ok() {
        let mut n = skipped;
        loop {
            n += 1;
            let text = line.trim();
            if !text.is_empty() {
                let d = match serde_json::from_str::<Value>(text) {
                    Ok(v) => doc(n, v),
                    Err(e) => Doc {
                        n,
                        text: Err(e.to_string()),
                    },
                };
                if !out.push(d) {
                    return Ok(DocLayout::Lines);
                }
            }
            line.clear();
            if file.read_line(&mut line)? == 0 {
                break;
            }
        }
        out.flush();
        return Ok(DocLayout::Lines);
    }
    let stream =
        serde_json::Deserializer::from_reader(Cursor::new(line).chain(file)).into_iter::<Value>();
    let mut n = 0;
    for v in stream {
        n += 1;
        let d = match v {
            Ok(v) => doc(n, v),
            Err(e) => {
                out.push(Doc {
                    n,
                    text: Err(e.to_string()),
                });
                break;
            }
        };
        if !out.push(d) {
            return Ok(DocLayout::Stream);
        }
    }
    out.flush();
    Ok(DocLayout::Stream)
}
