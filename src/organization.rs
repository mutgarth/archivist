use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
pub const FOLDERS: &[&str] = &[
    "Natureza e paisagens",
    "Animais",
    "Pessoas e encontros",
    "Comida e cozinha",
    "Trabalho e tecnologia",
    "Esportes e movimento",
    "Arte e música",
    "Lugares e arquitetura",
    "Objetos e documentos",
    "Revisar",
];
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Organization {
    pub folder: String,
    pub tags: Vec<String>,
    pub reason: String,
    pub confidence: f64,
    pub model: String,
    pub version: String,
    pub analysis_hash: String,
}
impl Organization {
    pub fn validate(&self) -> Result<()> {
        ensure!(FOLDERS.contains(&self.folder.as_str()), "invalid_folder");
        ensure!(
            self.confidence.is_finite() && (0.0..=1.0).contains(&self.confidence),
            "invalid_confidence"
        );
        ensure!(
            self.confidence >= 0.65 || self.folder == "Revisar",
            "requires_review"
        );
        ensure!(
            self.tags.len() <= 8
                && self.tags.iter().all(|s| !s.is_empty()
                    && s.chars().count() <= 40
                    && !s.chars().any(char::is_control)),
            "invalid_tags"
        );
        ensure!(
            !self.reason.is_empty() && self.reason.chars().count() <= 400,
            "invalid_reason"
        );
        ensure!(
            !self.model.is_empty() && self.model.len() <= 200 && self.version == "folders-v1",
            "invalid_model"
        );
        ensure!(
            self.analysis_hash.len() == 64
                && self.analysis_hash.bytes().all(|c| c.is_ascii_hexdigit()),
            "invalid_analysis_hash"
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Organization {
        Organization {
            folder: "Animais".into(),
            tags: vec!["gato".into()],
            reason: "Gato visível".into(),
            confidence: 0.9,
            model: "local".into(),
            version: "folders-v1".into(),
            analysis_hash: "a".repeat(64),
        }
    }
    #[test]
    fn rejects_paths_and_low_confidence() {
        let mut o = sample();
        assert!(o.validate().is_ok());
        o.folder = "../../x".into();
        assert!(o.validate().is_err());
        o.folder = "Animais".into();
        o.confidence = 0.2;
        assert!(o.validate().is_err());
        o.folder = "Revisar".into();
        assert!(o.validate().is_ok());
    }
}
