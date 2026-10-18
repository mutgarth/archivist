use crate::media;
use anyhow::{Result, ensure};
use cap_std::fs::Dir;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    os::fd::AsRawFd,
    process::{Command, Stdio},
};

pub struct Prepared {
    pub storage: String,
    pub sha256: String,
    pub mime: &'static str,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    pub metadata: Value,
}
fn fd_path(fd: &impl AsRawFd) -> String {
    format!("/proc/{}/fd/{}", std::process::id(), fd.as_raw_fd())
}
fn command(tool: &str, seconds: &str) -> Command {
    let mut c = Command::new("timeout");
    c.args([
        "--kill-after=5",
        seconds,
        &std::env::var(format!("{}_BIN", tool.to_uppercase())).unwrap_or(tool.into()),
    ]);
    c.stdin(Stdio::null());
    c
}
fn digest(mut file: cap_std::fs::File) -> Result<String> {
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub fn prepare(dir: &Dir, path: &str) -> Result<Prepared> {
    crate::domain::validate_relative(path)?;
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "mp4" | "mov" | "m4v") {
        let image = media::inspect(media::read(dir, path, false)?)?;
        let storage = media::persist(dir, &image)?;
        return Ok(Prepared {
            storage,
            sha256: image.sha256,
            mime: image.mime,
            bytes: image.bytes.len() as u64,
            width: image.width,
            height: image.height,
            metadata: json!({}),
        });
    }
    let mut source = media::open(dir, path, false)?;
    let before = source.metadata()?;
    ensure!(
        before.is_file() && before.len() > 0 && before.len() <= 64 * 1024 * 1024 * 1024,
        "invalid_video_size"
    );
    dir.create_dir_all(".archivist/originals")?;
    dir.create_dir_all(".archivist/previews")?;
    let temporary = format!(".archivist/originals/{}.tmp", uuid::Uuid::new_v4());
    let result = (|| -> Result<Prepared> {
        let mut options = cap_std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut dest = dir.open_with(&temporary, &options)?;
        let mut hash = Sha256::new();
        let mut total = 0u64;
        let mut buf = vec![0u8; 1024 * 1024];
        loop {
            let n = source.read(&mut buf)?;
            if n == 0 {
                break;
            }
            total += n as u64;
            ensure!(total <= before.len(), "source_changed");
            dest.write_all(&buf[..n])?;
            hash.update(&buf[..n]);
        }
        dest.sync_all()?;
        let after = source.metadata()?;
        ensure!(
            total == before.len()
                && before.len() == after.len()
                && before.modified()? == after.modified()?,
            "source_changed"
        );
        let sha256 = hex::encode(hash.finalize());
        ensure!(
            digest(dir.open(&temporary)?)? == sha256,
            "copy_verification_failed"
        );
        let snapshot = dir.open(&temporary)?;
        let output = command("ffprobe", "60")
            .args([
                "-v",
                "error",
                "-protocol_whitelist",
                "file",
                "-f",
                "mov",
                "-show_format",
                "-show_streams",
                "-of",
                "json",
                &fd_path(&snapshot),
            ])
            .output()?;
        ensure!(
            output.status.success() && output.stdout.len() < 1024 * 1024,
            "invalid_video"
        );
        let mut metadata: Value = serde_json::from_slice(&output.stdout)?;
        // The descriptor is an implementation detail, never a catalog path.
        metadata["format"]
            .as_object_mut()
            .map(|m| m.remove("filename"));
        let stream = metadata["streams"]
            .as_array()
            .and_then(|v| {
                v.iter()
                    .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
            })
            .ok_or_else(|| anyhow::anyhow!("no_video"))?;
        let width = stream["width"].as_u64().unwrap_or(0) as u32;
        let height = stream["height"].as_u64().unwrap_or(0) as u32;
        let duration = metadata["format"]["duration"]
            .as_str()
            .unwrap_or("0")
            .parse::<f64>()?;
        ensure!(
            width > 0
                && height > 0
                && width <= 8192
                && height <= 8192
                && duration.is_finite()
                && duration > 0.0
                && duration <= 86400.0,
            "unsupported_video"
        );
        let storage = format!(".archivist/originals/{sha256}");
        if dir.try_exists(&storage)? {
            ensure!(
                digest(dir.open(&storage)?)? == sha256,
                "stored_content_mismatch"
            );
            dir.remove_file(&temporary)?;
        } else {
            dir.rename(&temporary, dir, &storage)?;
        }
        let poster = format!(".archivist/previews/{sha256}.jpg");
        derive(dir, &storage, &poster, false, duration.min(1.0) / 2.0)?;
        Ok(Prepared {
            storage,
            sha256,
            mime: if ext == "mov" {
                "video/quicktime"
            } else {
                "video/mp4"
            },
            bytes: total,
            width,
            height,
            metadata,
        })
    })();
    if result.is_err() {
        let _ = dir.remove_file(&temporary);
    }
    result
}
fn derive(dir: &Dir, storage: &str, path: &str, playback: bool, seek: f64) -> Result<()> {
    media::check_storage()?;
    if dir.try_exists(path)? {
        return Ok(());
    }
    let source = media::open(dir, storage, true)?;
    let temp = format!("{path}.{}.tmp", uuid::Uuid::new_v4());
    let mut options = cap_std::fs::OpenOptions::new();
    options.read(true).write(true).create_new(true);
    let target = dir.open_with(&temp, &options)?;
    let result = (|| -> Result<()> {
        let mut c = command("ffmpeg", if playback { "3600" } else { "60" });
        c.args([
            "-nostdin",
            "-v",
            "error",
            "-threads",
            "2",
            "-filter_threads",
            "1",
            "-protocol_whitelist",
            "file",
        ]);
        if !playback {
            c.args(["-ss", &seek.to_string()]);
        }
        c.args(["-f", "mov", "-i", &fd_path(&source), "-map", "0:v:0"]);
        if playback {
            c.args([
                "-map",
                "0:a:0?",
                "-vf",
                "scale=1280:720:force_original_aspect_ratio=decrease:force_divisible_by=2:reset_sar=1",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "25",
                "-pix_fmt",
                "yuv420p",
                "-threads",
                "2",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-map_metadata",
                "-1",
                "-movflags",
                "+faststart",
                "-f",
                "mp4",
            ]);
        } else {
            c.args([
                "-frames:v",
                "1",
                "-vf",
                "scale=1024:1024:force_original_aspect_ratio=decrease:reset_sar=1",
                "-q:v",
                "3",
                "-f",
                "image2",
            ]);
        }
        let status = c
            .args(["-y", &fd_path(&target)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        ensure!(
            status.success() && target.metadata()?.len() > 0,
            "derivative_failed"
        );
        target.sync_all()?;
        media::check_storage()?;
        dir.rename(&temp, dir, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = dir.remove_file(&temp);
    }
    result
}
pub fn playback(dir: &Dir, storage: &str, hash: &str) -> Result<()> {
    ensure!(
        digest(media::open(dir, storage, true)?)? == hash,
        "original_changed"
    );
    dir.create_dir_all(".archivist/playback")?;
    derive(
        dir,
        storage,
        &format!(".archivist/playback/{hash}.mp4"),
        true,
        0.0,
    )
}

pub fn frame(dir: &Dir, storage: &str, hash: &str, at_ms: i64) -> Result<Vec<u8>> {
    ensure!(at_ms >= 0, "invalid_frame_time");
    dir.create_dir_all(".archivist/frames")?;
    let path = format!(".archivist/frames/{hash}-{at_ms}.jpg");
    derive(dir, storage, &path, false, at_ms as f64 / 1000.0)?;
    media::read(dir, &path, true)
}
pub fn audio_chunk(dir: &Dir, storage: &str, start_ms: i64, length_ms: i64) -> Result<Vec<u8>> {
    ensure!(
        start_ms >= 0 && length_ms > 0 && length_ms <= 60000,
        "invalid_audio_chunk"
    );
    let source = media::open(dir, storage, true)?;
    let output = command("ffmpeg", "120")
        .args([
            "-v",
            "error",
            "-nostdin",
            "-threads",
            "2",
            "-protocol_whitelist",
            "file",
            "-ss",
            &(start_ms as f64 / 1000.0).to_string(),
            "-f",
            "mov",
            "-i",
            &fd_path(&source),
            "-t",
            &(length_ms as f64 / 1000.0).to_string(),
            "-map",
            "0:a:0",
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
            "-f",
            "wav",
            "pipe:1",
        ])
        .output()?;
    ensure!(
        output.status.success()
            && output.stdout.len() > 44
            && output.stdout.len() < 4 * 1024 * 1024,
        "audio_extraction_failed"
    );
    Ok(output.stdout)
}
