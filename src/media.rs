use anyhow::{Result, ensure};
use cap_std::{ambient_authority, fs::Dir};
use image::{ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
use std::{
    io::{Cursor, Read, Write},
    path::Path,
};

pub const MAX_BYTES: u64 = 20 * 1024 * 1024;
pub struct Media {
    pub bytes: Vec<u8>,
    pub preview: Vec<u8>,
    pub sha256: String,
    pub mime: &'static str,
    pub width: u32,
    pub height: u32,
}
pub fn root(path: &str) -> Result<Dir> {
    Ok(Dir::open_ambient_dir(path, ambient_authority())?)
}
pub fn check_storage() -> Result<()> {
    if let Ok(expected) = std::env::var("MEDIA_VOLUME_ID") {
        ensure!(!expected.trim().is_empty(), "empty_volume_id");
        let dir = root(&std::env::var("MEDIA_ROOT")?)?;
        ensure!(
            dir.read_to_string(".archivist-volume")?.trim() == expected,
            "wrong_storage_volume"
        );
    }
    Ok(())
}
pub fn open(dir: &Dir, path: &str, internal: bool) -> Result<cap_std::fs::File> {
    check_storage()?;
    if !internal {
        crate::domain::validate_relative(path)?;
    }
    let mut current = std::path::PathBuf::new();
    for component in Path::new(path).components() {
        current.push(component);
        ensure!(
            !dir.symlink_metadata(&current)?.file_type().is_symlink(),
            "symlink"
        );
    }
    let file = dir.open(path)?;
    ensure!(file.metadata()?.is_file(), "invalid_file");
    Ok(file)
}
pub fn read(dir: &Dir, path: &str, internal: bool) -> Result<Vec<u8>> {
    let file = open(dir, path, internal)?;
    let before = file.metadata()?;
    ensure!(
        before.is_file() && before.len() <= MAX_BYTES,
        "invalid_file"
    );
    let mut bytes = Vec::new();
    (&file).take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    ensure!(
        bytes.len() as u64 == before.len()
            && before.len() == after.len()
            && before.modified()? == after.modified()?,
        "file_changed"
    );
    Ok(bytes)
}
pub fn inspect(bytes: Vec<u8>) -> Result<Media> {
    let format = image::guess_format(&bytes)?;
    let mime = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::WebP => "image/webp",
        _ => anyhow::bail!("unsupported_image"),
    };
    let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(10000);
    limits.max_image_height = Some(10000);
    limits.max_alloc = Some(160 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode()?;
    ensure!(
        u64::from(image.width()) * u64::from(image.height()) <= 40_000_000,
        "image_too_large"
    );
    let mut preview = Cursor::new(Vec::new());
    image
        .thumbnail(1024, 1024)
        .to_rgb8()
        .write_to(&mut preview, ImageFormat::Jpeg)?;
    Ok(Media {
        sha256: hex::encode(Sha256::digest(&bytes)),
        bytes,
        preview: preview.into_inner(),
        mime,
        width: image.width(),
        height: image.height(),
    })
}
pub fn persist(dir: &Dir, media: &Media) -> Result<String> {
    dir.create_dir_all(".archivist/originals")?;
    dir.create_dir_all(".archivist/previews")?;
    let original = format!(".archivist/originals/{}", media.sha256);
    for (path, data) in [
        (&original, &media.bytes),
        (
            &format!(".archivist/previews/{}.jpg", media.sha256),
            &media.preview,
        ),
    ] {
        if dir.try_exists(path)? {
            ensure!(read(dir, path, true)? == *data, "stored_content_mismatch");
            continue;
        }
        let temporary = format!("{path}.{}.tmp", uuid::Uuid::new_v4());
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        // Publish only complete bytes; a crash can leave a temp file, never a partial original.
        let result = (|| -> Result<()> {
            let mut file = dir.open_with(&temporary, &options)?;
            file.write_all(data)?;
            file.sync_all()?;
            dir.rename(&temporary, dir, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = dir.remove_file(&temporary);
        }
        result?;
    }
    Ok(original)
}
