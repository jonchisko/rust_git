use std::{
    ffi::CStr,
    io::{BufRead, Read, Write},
};

use anyhow::Context;

use crate::objects::{Kind, Object};

pub fn invoke(name_only: bool, tree_hash: String) -> anyhow::Result<()> {
    let mut object = Object::read_object(&tree_hash).context("parse out blob object file")?;

    match object.kind {
        Kind::Tree => {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            let mut buf = Vec::new();
            let mut hashbuf = [0; 20];

            loop {
                buf.clear();

                let n_bytes_read = object
                    .reader
                    .read_until(0, &mut buf)
                    .context("read next tree object entry")?;

                if n_bytes_read == 0 {
                    break;
                }

                object
                    .reader
                    .read_exact(&mut hashbuf)
                    .context("read tree entry object hash")?;
                let hash = hex::encode(&hashbuf);

                let mode_and_line =
                    CStr::from_bytes_with_nul(&buf).context("invalid tree entry")?;
                let mut bits = mode_and_line.to_bytes().splitn(2, |&b| b == b' ');
                let mode = bits.next().expect("split always yields once");
                let name = bits
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("tree entry has no filename"))?;

                if name_only {
                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                } else {
                    let mode = std::str::from_utf8(mode).context("mode is awlays valid utf-8")?;

                    let object =
                        Object::read_object(&hash).context("read objecct for tree entry")?;

                    write!(stdout, "{mode:0>6} {} {hash}", object.kind)
                        .context("write tree entry meta to stdout")?;

                    stdout
                        .write_all(name)
                        .context("write tree entry name to stdout")?;
                }

                writeln!(stdout, "").context("write newline to stdout")?;

                buf.clear();
            }
        }
        _ => anyhow::bail!("do not know how to print '{}'", object.kind),
    }

    Ok(())
}
