use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use sha1::{Digest, Sha1};
use std::{
    ffi::CStr,
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    CatFile {
        #[clap(short = 'p')]
        pretty_print: bool,

        object_hash: String,
    },
    HashObject {
        #[clap(short = 'w')]
        write: bool,

        file: PathBuf,
    },
}

enum Kind {
    Blob,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Command::CatFile {
            pretty_print,
            object_hash,
        } => {
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
                anyhow::bail!(
                    ".git/objects file header did not start with a known header: '{header}'"
                );
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
                    let n = std::io::copy(&mut z, &mut stdout)
                        .context("write .git/objects file to stdout")?;

                    anyhow::ensure!(
                        n as usize == size,
                        ".git/object file did not have expected size 
                        (actual: {n}, expected: {size})."
                    );
                }
            }
        }
        Command::HashObject { write, file } => {
            let stat =
                std::fs::metadata(&file).with_context(|| format!("stat file{}", file.display()))?;

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

                let mut file = std::fs::File::open(file)
                    .with_context(|| format!("open {}", file.display()))?;
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
        }
    }

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
