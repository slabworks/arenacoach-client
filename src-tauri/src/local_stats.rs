use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::match_payload::{Match, MatchResult};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanionStats {
    pub games: u32,
    pub wins: u32,
    pub losses: u32,
    pub unknown: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocalStats {
    #[serde(default)]
    matches: HashMap<String, MatchResult>,
}

impl LocalStats {
    pub fn load(path: &PathBuf) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(self).unwrap_or_else(|_| b"{}".to_vec()),
        )
    }

    pub fn record(&mut self, assembled: &Match) -> bool {
        self.record_result(&assembled.client_match_id, assembled.result)
    }

    pub fn record_result(&mut self, id: &str, result: MatchResult) -> bool {
        if id.is_empty() || self.matches.get(id) == Some(&result) {
            return false;
        }
        self.matches.insert(id.to_string(), result);
        true
    }

    pub fn reset(&mut self) {
        self.matches.clear();
    }

    pub fn summary(&self) -> CompanionStats {
        let mut stats = CompanionStats::default();
        for result in self.matches.values() {
            stats.games += 1;
            match result {
                MatchResult::Win => stats.wins += 1,
                MatchResult::Loss => stats.losses += 1,
                MatchResult::Unknown => stats.unknown += 1,
            }
        }
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn match_with(id: &str, result: MatchResult) -> Match {
        Match {
            client_match_id: id.into(),
            event_id: "Constructed_BestOf1".into(),
            format: crate::match_payload::Format::Constructed,
            result,
            player_seat: 1,
            deck_grp_ids: vec![],
            timeline: vec![],
        }
    }

    #[test]
    fn records_each_processed_game_once() {
        let mut stats = LocalStats::default();
        assert!(stats.record(&match_with("a", MatchResult::Win)));
        assert!(!stats.record(&match_with("a", MatchResult::Win)));
        assert!(stats.record(&match_with("b", MatchResult::Loss)));
        assert_eq!(
            stats.summary(),
            CompanionStats {
                games: 2,
                wins: 1,
                losses: 1,
                unknown: 0,
            }
        );
    }

    #[test]
    fn updates_the_result_when_the_same_game_finishes() {
        let mut stats = LocalStats::default();
        stats.record(&match_with("a", MatchResult::Unknown));
        stats.record(&match_with("a", MatchResult::Win));
        assert_eq!(
            stats.summary(),
            CompanionStats {
                games: 1,
                wins: 1,
                losses: 0,
                unknown: 0,
            }
        );
    }

    #[test]
    fn reset_clears_the_all_time_record() {
        let mut stats = LocalStats::default();
        stats.record(&match_with("a", MatchResult::Win));
        stats.reset();
        assert_eq!(stats.summary(), CompanionStats::default());
        assert!(stats.record(&match_with("a", MatchResult::Loss)));
        assert_eq!(stats.summary().losses, 1);
    }

    #[test]
    fn persists_across_reloads() {
        let path = std::env::temp_dir().join(format!(
            "arenacoach-stats-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut stats = LocalStats::default();
        stats.record(&match_with("keep-me", MatchResult::Win));
        stats.save(&path).unwrap();
        let restored = LocalStats::load(&path);
        let _ = std::fs::remove_file(&path);
        assert_eq!(restored.summary().wins, 1);
        assert_eq!(restored.summary().games, 1);
    }
}
