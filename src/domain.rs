use anyhow::{Result, ensure};
use std::path::Path;

pub fn validate_relative(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty() && path.len() <= 1024 && !Path::new(path).is_absolute(),
        "invalid_path"
    );
    ensure!(!path.contains(['\\', '\0']), "invalid_path");
    ensure!(
        path.split('/')
            .all(|s| !s.is_empty() && s != "." && s != ".." && s != ".archivist"),
        "invalid_path"
    );
    Ok(())
}
pub fn validate_embedding(v: &[f32], dimension: usize) -> Result<()> {
    ensure!(
        dimension > 0 && v.len() == dimension && v.iter().all(|n| n.is_finite()),
        "invalid_vector"
    );
    let norm: f64 = v.iter().map(|n| f64::from(*n).powi(2)).sum();
    ensure!(norm > 0.0 && norm.is_finite(), "invalid_vector");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_nested_relative_image() {
        assert!(validate_relative("lote/café.png").is_ok());
    }
    #[test]
    fn rejects_unsafe_paths() {
        for p in [
            "",
            "/",
            "/tmp/a",
            "../a",
            "a/../b",
            ".",
            "a//b",
            "a/./b",
            "a\\b",
            ".archivist/a",
        ] {
            assert!(validate_relative(p).is_err(), "{p}");
        }
    }
    #[test]
    fn accepts_compatible_nonzero_vector() {
        assert!(validate_embedding(&[0.1, 0.2], 2).is_ok());
    }
    #[test]
    fn rejects_invalid_vectors() {
        for v in [vec![], vec![0.0, 0.0], vec![f32::NAN, 1.0], vec![1.0]] {
            assert!(validate_embedding(&v, 2).is_err());
        }
    }
}
