use anyhow::Context;
use flate2::read::ZlibDecoder;
use std::{
    ffi::CStr,
    io::{BufRead, BufReader, Read},
};

enum Kind {
    Blob,
}

pub fn invoke(pretty_print: bool, object_hash: String) -> anyhow::Result<()> {
    anyhow::ensure!(
        pretty_print,
        "mode must be given without -p, we don't support it yet"
    );

    // TODO: support shortest unique object hashes
    let f = std::fs::File::open(format!(
        ".git/objects/{}/{}",
        &object_hash[..2],
        &object_hash[2..]
    ))
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
        _ => anyhow::bail!("we do not know yet how to handle this kind '{kind}'"),
    };

    let size = size
        .parse::<usize>()
        .context(".git/objects file has invalid size: {size}")?;

    let mut z = LimitReader {
        reader: z,
        limit: size,
    };

    match kind {
        Kind::Blob => {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            let n =
                std::io::copy(&mut z, &mut stdout).context("write .git/objects file to stdout")?;

            anyhow::ensure!(
                n as usize == size,
                ".git/object file did not have expected size 
                (actual: {n}, expected: {size})."
            );
        }
    }

    Ok(())
}

struct LimitReader<R> {
    reader: R,
    limit: usize,
}

impl<R> Read for LimitReader<R>
where
    R: Read,
{
    fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.len() > self.limit {
            buf = &mut buf[..self.limit + 1];
        }

        let n = self.reader.read(buf)?;
        if n > self.limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "too many bytes",
            ));
        }

        self.limit -= n;

        Ok(n)
    }
}
