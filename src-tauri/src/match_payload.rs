use serde::{Deserialize, Serialize};

pub const MAX_TIMELINE_EVENTS: usize = 500;
pub const MAX_PAYLOAD_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Constructed,
    Limited,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchResult {
    Win,
    Loss,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    Me,
    Opponent,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Cast,
    Land,
    Attack,
    Damage,
    Life,
    Mulligan,
    Keep,
    Zone,
    Pass,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub t_ms: u64,
    pub turn: Option<u32>,
    pub phase: Option<String>,
    pub actor: Actor,
    pub kind: EventKind,
    pub card_ids: Vec<u32>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Match {
    pub client_match_id: String,
    pub event_id: String,
    pub format: Format,
    pub result: MatchResult,
    pub player_seat: u32,
    pub deck_grp_ids: Vec<u32>,
    pub timeline: Vec<TimelineEvent>,
}

impl Match {
    pub fn infer_format(event_id: &str) -> Format {
        let event = event_id.to_ascii_lowercase();
        if ["draft", "sealed", "limited"]
            .iter()
            .any(|needle| event.contains(needle))
        {
            Format::Limited
        } else if [
            "constructed",
            "ladder",
            "historic",
            "standard",
            "alchemy",
            "explorer",
            "pioneer",
            "modern",
            "timeless",
            "brawl",
            "play",
        ]
        .iter()
        .any(|needle| event.contains(needle))
        {
            Format::Constructed
        } else {
            Format::Unknown
        }
    }

    pub fn cap_timeline(&mut self) {
        if self.timeline.len() <= MAX_TIMELINE_EVENTS {
            return;
        }
        let keep_head = MAX_TIMELINE_EVENTS - 20;
        let mut kept = self.timeline[..keep_head].to_vec();
        kept.extend_from_slice(&self.timeline[self.timeline.len() - 20..]);
        self.timeline = kept;
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut payload = self.clone();
        payload.cap_timeline();
        let mut bytes = serde_json::to_vec(&payload)?;
        while bytes.len() > MAX_PAYLOAD_BYTES && payload.timeline.len() > 20 {
            let drop_count = (payload.timeline.len() / 4).max(1);
            payload.timeline.drain(20..20 + drop_count);
            bytes = serde_json::to_vec(&payload)?;
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructed_and_limited_from_event_id() {
        assert_eq!(
            Match::infer_format("Constructed_BestOf1"),
            Format::Constructed
        );
        assert_eq!(Match::infer_format("PremierDraft_ECL"), Format::Limited);
        assert_eq!(Match::infer_format("SomethingElse"), Format::Unknown);
    }
}
