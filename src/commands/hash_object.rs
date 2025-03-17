use anyhow::Context;
use flate2::{write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn invoke(write: bool, file: &PathBuf) -> anyhow::Result<()> {
    let stat = std::fs::metadata(&file).with_context(|| format!("stat file{}", file.display()))?;

    fn write_blob<W>(size: u64, file: &Path, writer: W) -> anyhow::Result<String>
    where
        W: Write,
    {
        let e = ZlibEncoder::new(writer, Compression::default());

        let mut hash_writer = HashWriter {
            writer: e,
            hasher: Sha1::new(),
        };

        write!(hash_writer, "blob ")?;
        write!(hash_writer, "{}\0", size)?;

        let mut file =
            std::fs::File::open(file).with_context(|| format!("open {}", file.display()))?;
        std::io::copy(&mut file, &mut hash_writer).context("stream file into blob")?;

        let _ = hash_writer.writer.finish()?;
        let hash = hash_writer.hasher.finalize();

        Ok(hex::encode(hash))
    }

    let hash = if write {
        let tmp = "temporary";

        let hash = write_blob(
            stat.len(),
            &file,
            std::fs::File::create(tmp).context("construct temporary file for blob")?,
        )
        .context("write out blob object")?;

        fs::create_dir_all(format!(".git/objects/{}/", &hash[..2]))
            .context("creating subdir of .git/objects")?;
        fs::rename(tmp, format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))
            .context("move blob file into .git/objects")?;

        hash
    } else {
        write_blob(stat.len(), &file, std::io::sink()).context("write out blob object")?
    };

    println!("{hash}");

    Ok(())
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

impl<W> Write for HashWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n_written_bytes = self.writer.write(buf)?;
        self.hasher.update(&buf[..n_written_bytes]);

        Ok(n_written_bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
