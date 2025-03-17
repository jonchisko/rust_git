use anyhow::Context;

use crate::objects::{Kind, Object};

pub fn invoke(pretty_print: bool, object_hash: String) -> anyhow::Result<()> {
    anyhow::ensure!(
        pretty_print,
        "mode must be given without -p, we don't support it yet"
    );

    let mut object = Object::read_object(&object_hash).context("parse out blob object file")?;

    match object.kind {
        Kind::Blob => {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            let n = std::io::copy(&mut object.reader, &mut stdout)
                .context("write .git/objects file to stdout")?;

            anyhow::ensure!(
                n == object.expected_size,
                format!(
                    ".git/object file did not have expected size 
                        (actual: {}, expected: {}).",
                    n, object.expected_size
                )
            );
        }
        _ => anyhow::bail!("do not know how to print '{}'", object.kind),
    }

    Ok(())
}
