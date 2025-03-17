use anyhow::Context;
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    io::{BufRead, BufReader, Read},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Blob,
    Tree,
    Commit,
}

impl std::fmt::Display for Kind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Kind::Blob => write!(f, "blob"),
            Kind::Tree => write!(f, "tree"),
            Kind::Commit => write!(f, "commit"),
        }
    }
}

pub struct Object<R> {
    pub kind: Kind,
    pub expected_size: u64,
    pub reader: R,
}

impl Object<()> {
    pub fn read_object(hash: &str) -> anyhow::Result<Object<impl BufRead>> {
        // TODO: support shortest unique object hashes
        let f = std::fs::File::open(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
            .context("open in .git/objects")?;

        let z = ZlibDecoder::new(f);
        let mut z = BufReader::new(z);
        let mut buf = Vec::new();
        z.read_until(0u8, &mut buf)
            .context("read header from .git/objects")?;

        let header = CStr::from_bytes_with_nul(&buf)
            .expect("known that there is exactly one nul and at the end");
        let header = header
            .to_str()
            .context(".git/objects file header isn't valid UTF-8")?;

        let Some((kind, size)) = header.split_once(' ') else {
            anyhow::bail!(".git/objects file header did not start with a known header: '{header}'");
        };

        let kind = match kind {
            "blob" => Kind::Blob,
            "tree" => Kind::Tree,
            "commit" => Kind::Commit,
            _ => anyhow::bail!("we do not know yet how to handle this kind '{kind}'"),
        };

        let size = size
            .parse::<usize>()
            .context(".git/objects file has invalid size: {size}")?;

        let z = z.take(size as u64);

        Ok(Object {
            kind,
            expected_size: size as u64,
            reader: z,
        })
    }
}
