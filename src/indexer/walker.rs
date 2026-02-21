use std::path::{Path, PathBuf};

use glob::Pattern;
use walkdir::WalkDir;

/// Collect all indexable files under `root`.
///
/// If `include` globs are provided, only files matching at least one pattern are kept.
/// If no include globs are given, files whose extension is in `default_exts` are kept.
/// Files matching any `exclude` glob are always dropped.
/// Hidden directories (starting with `.`) are skipped.
pub fn collect_files(
    root: &Path,
    include: &[String],
    exclude: &[String],
    default_exts: &[String],
) -> Vec<PathBuf> {
    let include_patterns: Vec<Pattern> = include
        .iter()
        .filter_map(|g| Pattern::new(g).ok())
        .collect();
    let exclude_patterns: Vec<Pattern> = exclude
        .iter()
        .filter_map(|g| Pattern::new(g).ok())
        .collect();

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_str().unwrap_or("");
                if name.starts_with('.') {
                    return false;
                }
                // Prune directories covered by any exclude pattern
                let rel = e.path().strip_prefix(root).unwrap_or(e.path());
                let probe = format!("{}/x", rel.to_string_lossy());
                if exclude_patterns.iter().any(|p| p.matches(&probe)) {
                    return false;
                }
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.path();

            // Get path relative to root for glob matching
            let rel = path.strip_prefix(root).unwrap_or(path);
            let rel_str = rel.to_string_lossy();

            // Apply exclude patterns
            for pat in &exclude_patterns {
                if pat.matches(&rel_str) {
                    return None;
                }
            }

            // Apply include patterns or default extension filter
            if !include_patterns.is_empty() {
                let matched = include_patterns.iter().any(|p| p.matches(&rel_str));
                if !matched {
                    return None;
                }
            } else {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if !default_exts.iter().any(|e| e == ext) {
                    return None;
                }
            }

            Some(path.to_path_buf())
        })
        .collect()
}
