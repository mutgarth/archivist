use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub kind: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub embedding: Vec<f32>,
}
pub fn validate(segments: &[Segment], duration_ms: i64, dimension: usize) -> Result<()> {
    ensure!(
        !segments.is_empty() && segments.len() <= 4000,
        "invalid_segment_count"
    );
    ensure!(
        segments.iter().any(|s| s.kind == "visual"),
        "missing_visual_samples"
    );
    for s in segments {
        ensure!(
            matches!(s.kind.as_str(), "visual" | "speech")
                && s.start_ms >= 0
                && s.end_ms > s.start_ms
                && s.end_ms <= duration_ms,
            "invalid_segment_time"
        );
        ensure!(
            !s.text.trim().is_empty() && s.text.len() <= 8000,
            "invalid_segment_text"
        );
        crate::domain::validate_embedding(&s.embedding, dimension)?;
    }
    Ok(())
}
pub fn samples(duration_ms: i64) -> Vec<i64> {
    let count = ((duration_ms + 29999) / 30000 + 1).clamp(1, 24);
    if duration_ms <= 1000 {
        return vec![duration_ms / 2];
    }
    (0..count)
        .map(|i| (i * duration_ms / (count - 1)).clamp(500, duration_ms - 500))
        .collect()
}
pub fn speech(value: &Value, offset: i64, length: i64) -> Result<Vec<Segment>> {
    let array = value["segments"]
        .as_array()
        .context("missing_speech_segments")?;
    let mut out = Vec::new();
    for item in array {
        if item["no_speech_prob"].as_f64().unwrap_or(0.0) > 0.6 {
            continue;
        }
        let text = item["text"].as_str().context("missing_speech_text")?.trim();
        if text.is_empty() {
            continue;
        }
        let start = item["start"].as_f64().context("missing_speech_start")?;
        let end = item["end"].as_f64().context("missing_speech_end")?;
        ensure!(
            start.is_finite()
                && end.is_finite()
                && start >= 0.0
                && end > start
                && text.len() <= 8000,
            "invalid_speech_segment"
        );
        let start_ms = (start * 1000.0).round() as i64;
        let end_ms = ((end * 1000.0).round() as i64).min(length);
        if start_ms >= end_ms {
            continue;
        }
        out.push(Segment {
            kind: "speech".into(),
            start_ms: offset + start_ms,
            end_ms: offset + end_ms,
            text: text.into(),
            embedding: vec![],
        });
    }
    Ok(out)
}
pub async fn analyze(
    ai: &crate::ai::Ai,
    dir: &cap_std::fs::Dir,
    storage: &str,
    hash: &str,
    metadata: &Value,
) -> Result<(Vec<Segment>, Option<String>)> {
    let duration_ms = (metadata["format"]["duration"]
        .as_str()
        .context("missing_duration")?
        .parse::<f64>()?
        * 1000.0)
        .round() as i64;
    ensure!(
        duration_ms > 0 && duration_ms <= 3600000,
        "timeline_limit_one_hour"
    );
    let has_audio = metadata["streams"]
        .as_array()
        .is_some_and(|s| s.iter().any(|v| v["codec_type"] == "audio"));
    let mut segments = Vec::new();
    let mut transcription_model = None;
    if has_audio {
        let url = std::env::var("WHISPER_URL").context("missing_WHISPER_URL")?;
        let model = std::env::var("WHISPER_MODEL").context("missing_WHISPER_MODEL")?;
        ensure!(
            !model.is_empty() && model.len() <= 200,
            "invalid_whisper_model"
        );
        for offset in (0..duration_ms).step_by(60000) {
            let length = (duration_ms - offset).min(60000);
            let root = dir.try_clone()?;
            let path = storage.to_owned();
            let wav = tokio::task::spawn_blocking(move || {
                crate::video::audio_chunk(&root, &path, offset, length)
            })
            .await??;
            let form = reqwest::multipart::Form::new()
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(wav)
                        .file_name("audio.wav")
                        .mime_str("audio/wav")?,
                )
                .text("response_format", "verbose_json")
                .text("language", "auto")
                .text("temperature", "0")
                .text("token_timestamps", "false");
            let value: Value = ai
                .client
                .post(format!("{}/inference", url.trim_end_matches('/')))
                .multipart(form)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            segments.extend(speech(&value, offset, length)?);
            tracing::info!(offset_ms = offset, "audio chunk transcribed");
        }
        transcription_model = Some(model);
    }
    let points = samples(duration_ms);
    for (index, at) in points.into_iter().enumerate() {
        let root = dir.try_clone()?;
        let path = storage.to_owned();
        let hash = hash.to_owned();
        let jpeg =
            tokio::task::spawn_blocking(move || crate::video::frame(&root, &path, &hash, at))
                .await??;
        let text = ai.describe(&jpeg).await?;
        segments.push(Segment {
            kind: "visual".into(),
            start_ms: at,
            end_ms: (at + 1).min(duration_ms),
            text,
            embedding: vec![],
        });
        tracing::info!(sample = index + 1, at_ms = at, "visual moment described");
    }
    ensure!(segments.len() <= 4000, "too_many_segments");
    for segment in &mut segments {
        segment.embedding = ai.embed(&segment.text, false).await?;
    }
    segments.sort_by_key(|s| s.start_ms);
    validate(&segments, duration_ms, ai.dimension)?;
    Ok((segments, transcription_model))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn sampling_covers_both_ends_and_is_bounded() {
        assert_eq!(samples(175680).len(), 7);
        assert_eq!(*samples(175680).last().unwrap(), 175180);
        assert_eq!(samples(3600000).len(), 24);
        assert_eq!(samples(600), vec![300]);
    }
    #[test]
    fn speech_offsets_seconds_and_filters_silence() {
        let s=speech(&json!({"segments":[{"text":"Olá","start":2.4,"end":5.0},{"text":"ruído","start":5.0,"end":8.0,"no_speech_prob":0.9}]}),60000,10000).unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].start_ms, 62400);
        assert_eq!(s[0].end_ms, 65000);
    }
    #[test]
    fn invalid_speech_is_not_silently_accepted() {
        assert!(speech(&json!({"text":"missing timestamps"}), 0, 1000).is_err());
        assert!(
            speech(
                &json!({"segments":[{"text":"x","start":-1,"end":1}]}),
                0,
                1000
            )
            .is_err()
        );
    }
}
