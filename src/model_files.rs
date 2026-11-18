//! Content hashes are computed once and cached by local file identity, size and
//! modification time. Model artifacts are trusted/admin-managed and must not be
//! modified while jobs run. Set ELA_DISABLE_MODEL_CACHE=1 to force full hashing.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
};

#[derive(Deserialize, Serialize, PartialEq)]
struct Identity {
    size: u64,
    modified_ns: u128,
    inode: u64,
    device: u64,
}
#[derive(Deserialize, Serialize)]
struct Cached {
    identity: Identity,
    digest: String,
}
fn identity(path: &Path) -> std::io::Result<Identity> {
    let metadata = std::fs::metadata(path)?;
    #[cfg(unix)]
    let (inode, device) = {
        use std::os::unix::fs::MetadataExt;
        (metadata.ino(), metadata.dev())
    };
    #[cfg(not(unix))]
    let (inode, device) = (0, 0);
    Ok(Identity {
        size: metadata.len(),
        modified_ns: metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(std::io::Error::other)?
            .as_nanos(),
        inode,
        device,
    })
}
pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let path = path.canonicalize()?;
    let before = identity(&path)?;
    let enabled = std::env::var("ELA_DISABLE_MODEL_CACHE").as_deref() != Ok("1");
    let base = std::env::var_os("ELA_WORK_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| ".ela-work".into())
        .join("model-cache");
    let key = format!(
        "{:x}.json",
        Sha256::digest(path.as_os_str().as_encoded_bytes())
    );
    let cache = base.join(key);
    if enabled
        && let Ok(bytes) = std::fs::read(&cache)
        && let Ok(cached) = serde_json::from_slice::<Cached>(&bytes)
        && cached.identity == before
        && cached.digest.len() == 64
        && cached.digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Ok(cached.digest);
    }
    let mut file = std::fs::File::open(&path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if identity(&path)? != before {
        return Err(std::io::Error::other("Model changed during fingerprinting"));
    }
    let digest = format!("{:x}", hash.finalize());
    if enabled
        && std::fs::create_dir_all(&base).is_ok()
        && let Ok(mut temporary) = tempfile::NamedTempFile::new_in(&base)
        && let Ok(json) = serde_json::to_vec(&Cached {
            identity: before,
            digest: digest.clone(),
        })
        && temporary.write_all(&json).is_ok()
    {
        let _ = temporary.persist(cache);
    }
    Ok(digest)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reuses_content_hash_and_invalidates_changed_artifacts() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model");
        std::fs::write(&path, b"first").unwrap();
        let first = hash_file(&path).unwrap();
        assert_eq!(first, hash_file(&path).unwrap());
        std::fs::write(&path, b"different model contents").unwrap();
        assert_ne!(first, hash_file(&path).unwrap());
    }
}
