//! Comparison of Hollow Knight mod version strings.
//!
//! Mod versions are dot-separated numeric tuples like `1.0.0.0` (not semver), and the
//! modding API uses a single integer like `77`. Both are handled by comparing the
//! numeric components left-to-right, zero-padding the shorter one.

use std::cmp::Ordering;

/// Parse a version string into its numeric components (non-numeric parts become 0).
pub fn parse(v: &str) -> Vec<u64> {
    v.trim()
        .split('.')
        .map(|p| p.trim().parse().unwrap_or(0))
        .collect()
}

/// Compare two version strings component-wise.
pub fn cmp(a: &str, b: &str) -> Ordering {
    let (a, b) = (parse(a), parse(b));
    let n = a.len().max(b.len());
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

/// Whether `candidate` is strictly newer than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    cmp(candidate, current) == Ordering::Greater
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_four_part_versions() {
        assert!(is_newer("1.0.0.1", "1.0.0.0"));
        assert!(is_newer("1.2.0.0", "1.1.9.9"));
        assert!(!is_newer("1.0.0.0", "1.0.0.0"));
        assert!(!is_newer("0.9.0.0", "1.0.0.0"));
    }

    #[test]
    fn handles_differing_lengths() {
        assert_eq!(cmp("1.0", "1.0.0.0"), Ordering::Equal);
        assert!(is_newer("1.0.1", "1.0"));
    }

    #[test]
    fn handles_integer_api_versions() {
        assert!(is_newer("78", "77"));
        assert!(!is_newer("77", "77"));
    }
}
