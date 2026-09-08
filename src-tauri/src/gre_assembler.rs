use std::collections::HashMap;

use serde_json::Value;

use crate::log_follower::{extract_json_values, LogEntry};
use crate::match_payload::{Actor, EventKind, Match, MatchResult, TimelineEvent};

const VISIBILITY_HIDDEN: &str = "Visibility_Hidden";
const GAME_STAGE_OVER: &str = "GameStage_GameOver";
const MATCH_STATE_COMPLETE: &str = "MatchState_MatchComplete";
const MATCH_STATE_GAME_COMPLETE: &str = "MatchState_GameComplete";

#[derive(Debug, Clone)]
struct GameObject {
    grp_id: Option<u32>,
    owner_seat: Option<u32>,
    visibility: Option<String>,
}

#[derive(Debug, Default)]
pub struct GreAssembler {
    match_id: Option<String>,
    event_id: Option<String>,
    player_seat: Option<u32>,
    deck_grp_ids: Vec<u32>,
    objects: HashMap<u32, GameObject>,
    timeline: Vec<TimelineEvent>,
    turn: Option<u32>,
    phase: Option<String>,
    next_t_ms: u64,
    result: MatchResult,
    in_match: bool,
}

impl GreAssembler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn in_match(&self) -> bool {
        self.in_match && self.match_id.is_some()
    }

    #[cfg(test)]
    fn has_object(&self, instance_id: u32) -> bool {
        self.objects.contains_key(&instance_id)
    }

    pub fn push(&mut self, entry: &LogEntry) -> Option<Match> {
        let mut completed = None;
        for value in extract_json_values(&entry.body) {
            if let Some(finished) = self.push_json(&value) {
                completed = Some(finished);
            }
        }
        completed
    }

    fn push_json(&mut self, value: &Value) -> Option<Match> {
        if let Some(room) = value.get("matchGameRoomStateChangedEvent") {
            self.handle_room(room);
            return None;
        }
        if let Some(gre) = value
            .get("greToClientEvent")
            .or_else(|| value.get("greToClientMessages").map(|_| value))
        {
            return self.handle_gre(gre);
        }
        if value.get("clientToGREMessage").is_some() || value.get("clientToGreMessage").is_some() {
            self.handle_client_to_gre(value);
        }
        None
    }

    fn handle_room(&mut self, room: &Value) {
        let info = room.get("gameRoomInfo").unwrap_or(room);
        let state = info.get("stateType").and_then(Value::as_str).unwrap_or("");
        let config = info.get("gameRoomConfig").unwrap_or(info);
        let match_id = string_field(config, &["matchId", "matchID"]);
        let event_id = string_field(config, &["eventId", "eventID"]);

        if state.contains("Playing") {
            if let Some(id) = match_id {
                if self.match_id.as_deref() != Some(id.as_str()) {
                    *self = Self::default();
                    self.match_id = Some(id);
                    self.event_id = event_id;
                    self.in_match = true;
                }
            }
        }
        if state.contains("MatchCompleted") {
            self.in_match = false;
        }
    }

    fn handle_gre(&mut self, gre: &Value) -> Option<Match> {
        let messages = gre
            .get("greToClientEvent")
            .and_then(|event| event.get("greToClientMessages"))
            .or_else(|| gre.get("greToClientMessages"))
            .and_then(Value::as_array)?;

        self.maybe_set_local_seat(messages);

        let mut completed = None;
        for message in messages {
            let msg_type = message.get("type").and_then(Value::as_str).unwrap_or("");
            match msg_type {
                "GREMessageType_ConnectResp" => self.handle_connect(message),
                "GREMessageType_GameStateMessage"
                | "GREMessageType_QueuedGameStateMessage" => {
                    if let Some(finished) = self.handle_game_state(message) {
                        completed = Some(finished);
                    }
                }
                _ => {}
            }
        }
        completed
    }

    fn maybe_set_local_seat(&mut self, messages: &[Value]) {
        if self.player_seat.is_some() {
            return;
        }
        if let Some(connect) = messages.iter().find(|message| {
            message.get("type").and_then(Value::as_str) == Some("GREMessageType_ConnectResp")
        }) {
            if let Some(seat) = singleton_seat(connect) {
                self.player_seat = Some(seat);
            }
            return;
        }
        for message in messages {
            let msg_type = message.get("type").and_then(Value::as_str).unwrap_or("");
            if matches!(
                msg_type,
                "GREMessageType_UIMessage"
                    | "GREMessageType_TimerStateMessage"
                    | "GREMessageType_SetSettingsResp"
            ) {
                continue;
            }
            if let Some(seat) = singleton_seat(message) {
                self.player_seat = Some(seat);
                return;
            }
        }
    }

    fn handle_connect(&mut self, message: &Value) {
        if let Some(seat) = singleton_seat(message) {
            self.player_seat = Some(seat);
        }
        let Some(connect) = message.get("connectResp") else {
            return;
        };
        if let Some(cards) = connect
            .get("deckMessage")
            .and_then(|deck| deck.get("deckCards"))
            .and_then(Value::as_array)
        {
            self.deck_grp_ids = cards.iter().filter_map(json_u32).collect();
        }
        self.in_match = true;
    }

    fn handle_game_state(&mut self, message: &Value) -> Option<Match> {
        let gsm = message
            .get("gameStateMessage")
            .or_else(|| message.get("queuedGameStateMessage"))?;
        if let Some(info) = gsm.get("gameInfo") {
            if let Some(id) = string_field(info, &["matchID", "matchId"]) {
                if self.match_id.is_none() {
                    self.match_id = Some(id);
                    self.in_match = true;
                }
            }
        }
        if let Some(turn) = gsm.get("turnInfo") {
            if let Some(number) = turn.get("turnNumber").and_then(json_u32) {
                self.turn = Some(number);
            }
            if let Some(phase) = turn.get("phase").and_then(Value::as_str) {
                self.phase = Some(phase.to_string());
            }
        }

        self.merge_objects(gsm.get("gameObjects"));

        let annotations = gsm
            .get("annotations")
            .and_then(Value::as_array)
            .into_iter()
            .chain(
                gsm.get("persistentAnnotations")
                    .and_then(Value::as_array)
                    .into_iter(),
            )
            .flatten();
        for annotation in annotations {
            self.push_annotation(annotation);
        }

        if let Some(deleted) = gsm.get("diffDeletedInstanceIds").and_then(Value::as_array) {
            for id in deleted.iter().filter_map(json_u32) {
                self.objects.remove(&id);
            }
        }

        let info = gsm.get("gameInfo")?;
        let stage = info.get("stage").and_then(Value::as_str).unwrap_or("");
        let match_state = info.get("matchState").and_then(Value::as_str).unwrap_or("");
        if stage != GAME_STAGE_OVER {
            return None;
        }
        if match_state == MATCH_STATE_COMPLETE {
            return None;
        }
        if match_state == MATCH_STATE_GAME_COMPLETE || match_state.is_empty() {
            self.result = result_from_info(info, self.player_seat);
            self.in_match = false;
            return self.emit_match();
        }
        None
    }

    fn merge_objects(&mut self, objects: Option<&Value>) {
        let Some(objects) = objects.and_then(Value::as_array) else {
            return;
        };
        for object in objects {
            let Some(instance_id) = object.get("instanceId").and_then(json_u32) else {
                continue;
            };
            let existing = self.objects.entry(instance_id).or_insert(GameObject {
                grp_id: None,
                owner_seat: None,
                visibility: None,
            });
            if let Some(grp_id) = object.get("grpId").and_then(json_u32) {
                existing.grp_id = Some(grp_id);
            }
            if let Some(owner) = object
                .get("ownerSeatId")
                .or_else(|| object.get("controllerSeatId"))
                .and_then(json_u32)
            {
                existing.owner_seat = Some(owner);
            }
            if let Some(visibility) = object.get("visibility").and_then(Value::as_str) {
                existing.visibility = Some(visibility.to_string());
            }
        }
    }

    fn handle_client_to_gre(&mut self, value: &Value) {
        let message = value
            .get("clientToGREMessage")
            .or_else(|| value.get("clientToGreMessage"))
            .unwrap_or(value);
        let msg_type = message.get("type").and_then(Value::as_str).unwrap_or("");
        if msg_type != "ClientMessageType_MulliganResp" {
            return;
        }
        let decision = message
            .get("mulliganResp")
            .and_then(|resp| resp.get("decision"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let (kind, note) = if decision.contains("Accept") || decision.contains("Keep") {
            (EventKind::Keep, "Keep")
        } else {
            (EventKind::Mulligan, "Mulligan")
        };
        let event = self.event(Actor::Me, kind, Vec::new(), note);
        self.timeline.push(event);
    }

    fn push_annotation(&mut self, annotation: &Value) {
        let types = annotation
            .get("type")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let type_name = types
            .first()
            .and_then(Value::as_str)
            .unwrap_or("");
        let details = annotation
            .get("details")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let affector = annotation.get("affectorId").and_then(json_u32);
        let affected = annotation
            .get("affectedIds")
            .and_then(Value::as_array)
            .map(|ids| ids.iter().filter_map(json_u32).collect::<Vec<_>>())
            .unwrap_or_default();
        let card_ids = self.visible_card_ids(&affected);
        let actor = self.actor_for(affector, &affected);
        let category = detail_string(&details, "category");

        let (kind, note) = match type_name {
            "AnnotationType_ZoneTransfer" => {
                let kind = match category.as_deref() {
                    Some("PlayLand") => EventKind::Land,
                    Some("CastSpell") => EventKind::Cast,
                    _ => EventKind::Zone,
                };
                (kind, category.unwrap_or_else(|| "ZoneTransfer".into()))
            }
            "AnnotationType_DamageDealt" => {
                let amount = detail_int(&details, "damage").unwrap_or(0);
                (EventKind::Damage, format!("damage {amount}"))
            }
            "AnnotationType_ModifiedLife" => {
                let amount = detail_int(&details, "life")
                    .or_else(|| detail_int(&details, "value"))
                    .unwrap_or(0);
                (EventKind::Life, format!("life {amount}"))
            }
            "AnnotationType_DeclaredAttackers" => (EventKind::Attack, "attack".into()),
            "AnnotationType_PhaseOrStepModified" => {
                if self.phase.as_deref() == Some("Phase_Ending") {
                    (EventKind::Pass, "pass".into())
                } else {
                    return;
                }
            }
            _ => return,
        };

        let event = self.event(actor, kind, card_ids, &note);
        self.timeline.push(event);
    }

    fn visible_card_ids(&self, instance_ids: &[u32]) -> Vec<u32> {
        instance_ids
            .iter()
            .filter_map(|id| self.objects.get(id))
            .filter(|object| object.visibility.as_deref() != Some(VISIBILITY_HIDDEN))
            .filter_map(|object| object.grp_id)
            .collect()
    }

    fn actor_for(&self, affector: Option<u32>, affected: &[u32]) -> Actor {
        let player_seat = self.player_seat;
        if let Some(id) = affector {
            if player_seat == Some(id) {
                return Actor::Me;
            }
            if let Some(object) = self.objects.get(&id) {
                if object.owner_seat == player_seat {
                    return Actor::Me;
                }
                if object.owner_seat.is_some() {
                    return Actor::Opponent;
                }
            }
            if player_seat.is_some() && matches!(id, 1 | 2) {
                return Actor::Opponent;
            }
        }
        if let Some(first) = affected.first().and_then(|id| self.objects.get(id)) {
            if first.owner_seat == player_seat {
                return Actor::Me;
            }
            if first.owner_seat.is_some() {
                return Actor::Opponent;
            }
        }
        Actor::Game
    }

    fn event(&mut self, actor: Actor, kind: EventKind, card_ids: Vec<u32>, note: &str) -> TimelineEvent {
        let t_ms = self.next_t_ms;
        self.next_t_ms += 1000;
        TimelineEvent {
            t_ms,
            turn: self.turn,
            phase: self.phase.clone(),
            actor,
            kind,
            card_ids,
            note: note.to_string(),
        }
    }

    fn emit_match(&self) -> Option<Match> {
        let client_match_id = self.match_id.clone()?;
        let event_id = self
            .event_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let mut assembled = Match {
            client_match_id,
            format: Match::infer_format(&event_id),
            event_id,
            result: self.result,
            player_seat: self.player_seat.unwrap_or(1),
            deck_grp_ids: self.deck_grp_ids.clone(),
            timeline: self.timeline.clone(),
        };
        assembled.cap_timeline();
        Some(assembled)
    }
}

fn singleton_seat(message: &Value) -> Option<u32> {
    let seats = message.get("systemSeatIds").and_then(Value::as_array)?;
    if seats.len() == 1 {
        json_u32(&seats[0])
    } else {
        None
    }
}

fn result_from_info(info: &Value, player_seat: Option<u32>) -> MatchResult {
    let Some(results) = info.get("results").and_then(Value::as_array) else {
        return MatchResult::Unknown;
    };
    let winning = results.iter().find_map(|result| {
        result.get("winningTeamId").and_then(json_u32)
    });
    match (winning, player_seat) {
        (Some(winner), Some(seat)) if winner == seat => MatchResult::Win,
        (Some(winner), Some(seat)) if winner != seat => MatchResult::Loss,
        _ => MatchResult::Unknown,
    }
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn json_u32(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .map(|n| n as u32)
        .or_else(|| value.as_i64().map(|n| n as u32))
        .or_else(|| value.as_str()?.parse().ok())
}

fn detail_string(details: &[Value], key: &str) -> Option<String> {
    details.iter().find_map(|detail| {
        if detail.get("key").and_then(Value::as_str) != Some(key) {
            return None;
        }
        detail
            .get("valueString")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    })
}

fn detail_int(details: &[Value], key: &str) -> Option<i64> {
    details.iter().find_map(|detail| {
        if detail.get("key").and_then(Value::as_str) != Some(key) {
            return None;
        }
        detail
            .get("valueInt32")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
            .and_then(Value::as_i64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_follower::{split_entries, LogEntry};
    use crate::match_payload::Format;

    fn entries_from(log: &str) -> Vec<LogEntry> {
        let (entries, remainder) = split_entries(log);
        assert!(remainder.is_empty(), "fixture must be complete: {remainder}");
        entries
    }

    fn assemble(log: &str) -> Option<Match> {
        let mut assembler = GreAssembler::new();
        let mut last = None;
        for entry in entries_from(log) {
            if let Some(finished) = assembler.push(&entry) {
                last = Some(finished);
            }
        }
        last
    }

    #[test]
    fn assembles_fixture_match_and_strips_identity() {
        let log = include_str!("../fixtures/match_bo1.log");
        let assembled = assemble(log).expect("game over should emit a match");
        assert_eq!(assembled.client_match_id, "match-fixture-001");
        assert_eq!(assembled.event_id, "Constructed_BestOf1");
        assert_eq!(assembled.format, Format::Constructed);
        assert_eq!(assembled.result, MatchResult::Win);
        assert_eq!(assembled.player_seat, 1);
        assert_eq!(assembled.deck_grp_ids.len(), 60);
        assert!(assembled.deck_grp_ids.contains(&68398));
        assert!(assembled.timeline.len() >= 20);
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Mulligan)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Keep)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Land && event.card_ids == vec![68398])
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Cast && event.actor == Actor::Me)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Cast && event.actor == Actor::Opponent)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Attack)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Damage)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Life)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Pass)
        );
        let json = serde_json::to_string(&assembled).unwrap();
        assert!(!json.contains("REDACTED_USER"));
        assert!(!json.contains("Alice#11111"));
        assert!(!json.contains("screenName"));
        assert!(!json.contains("sessionId"));
    }

    #[test]
    fn game_complete_emits_once_match_complete_is_ignored() {
        let log = include_str!("../fixtures/match_bo1.log");
        let mut assembler = GreAssembler::new();
        let mut emitted = 0;
        for entry in entries_from(log) {
            if assembler.push(&entry).is_some() {
                emitted += 1;
            }
        }
        assert_eq!(emitted, 1);
    }

    #[test]
    fn batched_game_state_messages_are_all_applied() {
        let log = include_str!("../fixtures/match_bo1.log");
        let assembled = assemble(log).unwrap();
        let kinds: Vec<_> = assembled.timeline.iter().map(|event| event.kind).collect();
        assert!(kinds.contains(&EventKind::Land));
        assert!(kinds.contains(&EventKind::Cast));
        assert!(kinds.contains(&EventKind::Attack));
        assert!(kinds.contains(&EventKind::Life));
    }

    #[test]
    fn incremental_delete_drops_object() {
        let log = include_str!("../fixtures/match_bo1.log");
        let mut assembler = GreAssembler::new();
        for entry in entries_from(log) {
            assembler.push(&entry);
        }
        assert!(!assembler.has_object(999));
    }

    #[test]
    fn hidden_opponent_cards_are_omitted() {
        let log = include_str!("../fixtures/hidden_cards.log");
        let assembled = assemble(log).expect("hidden fixture should complete");
        assert!(
            assembled
                .timeline
                .iter()
                .all(|event| event.card_ids.is_empty())
        );
    }

    #[test]
    fn missing_detailed_logs_emits_nothing() {
        let log = include_str!("../fixtures/no_detailed_logs.log");
        assert!(assemble(log).is_none());
    }
}
