use serde::Serialize;
use std::path::Path;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    pub used_bytes: u64,
    pub limit_bytes: u64,
    pub next_clip_bytes: u64,
    pub can_save: bool,
}
pub fn estimate(s: &crate::Settings) -> u64 {
    u64::from(s.bitrate) * u64::from(s.seconds) * 125_000 * 5 / 4 + 8_000_000
}
pub fn snapshot(root: &Path, limit_mb: u32, s: &crate::Settings) -> Result<Storage, String> {
    let used_bytes = crate::clips::list(root)?.iter().map(|c| c.bytes).sum();
    let limit_bytes = u64::from(limit_mb) * 1_000_000;
    let next_clip_bytes = estimate(s);
    Ok(Storage {
        used_bytes,
        limit_bytes,
        next_clip_bytes,
        can_save: fits(used_bytes, limit_bytes, next_clip_bytes),
    })
}
pub fn fits(used: u64, limit: u64, next: u64) -> bool {
    limit == 0 || used.saturating_add(next) <= limit
}
pub fn guard(root: &Path, limit_mb: u32, s: &crate::Settings) -> Result<(), String> {
    let current = snapshot(root, limit_mb, s)?;
    if current.can_save {
        Ok(())
    } else {
        Err("Not enough room in your clip-library budget for the next clip. Move clips or increase capacity in Storage settings. Existing clips were kept.".into())
    }
}
pub fn export_room(root: &Path, limit_mb: u32) -> Result<Option<u64>, String> {
    if limit_mb == 0 {
        return Ok(None);
    }
    let used = crate::clips::list(root)?
        .iter()
        .map(|c| c.bytes)
        .sum::<u64>();
    let remaining = (u64::from(limit_mb) * 1_000_000).saturating_sub(used);
    if remaining < 1_000_000 {
        return Err("Clip-library capacity reached. Move clips or increase your storage budget before exporting.".into());
    }
    Ok(Some(remaining))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_preflight_does_not_need_deleting_clips() {
        assert!(fits(100, 0, 1000));
        assert!(fits(100, 200, 100));
        assert!(!fits(101, 200, 100));
        assert!(!fits(u64::MAX, 200, 100));
    }
}
