use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

pub const RAM_TARGET: u64 = 1_000_000_000;
pub const GAME_MODE_THRESHOLD: u64 = 850_000_000;
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub bytes: u64,
    pub processes: u32,
    pub approximate: bool,
}
fn kilobytes(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let rest = line.strip_prefix(key)?;
        rest.split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
            .map(|n| n * 1024)
    })
}
fn descendants(root: u32, parents: &HashMap<u32, u32>) -> HashSet<u32> {
    let mut selected = HashSet::from([root]);
    loop {
        let before = selected.len();
        for (&pid, &parent) in parents {
            if selected.contains(&parent) {
                selected.insert(pid);
            }
        }
        if selected.len() == before {
            break;
        }
    }
    selected
}
pub fn sample(root: u32) -> Sample {
    let mut parents = HashMap::new();
    let mut rss = HashMap::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
                continue;
            };
            let Ok(status) = fs::read_to_string(entry.path().join("status")) else {
                continue;
            };
            if let Some(parent) = status.lines().find_map(|l| {
                l.strip_prefix("PPid:")
                    .and_then(|n| n.trim().parse::<u32>().ok())
            }) {
                parents.insert(pid, parent);
            }
            rss.insert(pid, kilobytes(&status, "VmRSS:").unwrap_or(0));
        }
    }
    let mut total = Sample::default();
    for pid in descendants(root, &parents) {
        let pss = fs::read_to_string(
            Path::new("/proc")
                .join(pid.to_string())
                .join("smaps_rollup"),
        )
        .ok()
        .and_then(|s| kilobytes(&s, "Pss:"));
        let value = pss.unwrap_or_else(|| {
            total.approximate = true;
            *rss.get(&pid).unwrap_or(&0)
        });
        if value > 0 {
            total.bytes += value;
            total.processes += 1;
        }
    }
    total
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_nested_children_once() {
        let map = HashMap::from([(2, 1), (3, 2), (4, 1), (5, 99)]);
        assert_eq!(descendants(1, &map), HashSet::from([1, 2, 3, 4]));
        assert_eq!(
            kilobytes("Pss: 123 kB\nPss_Anon: 9 kB", "Pss:"),
            Some(125952)
        );
    }
    #[test]
    fn can_measure_this_process() {
        let s = sample(std::process::id());
        assert!(s.bytes > 0);
        assert!(s.processes >= 1);
    }
}
