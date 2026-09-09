use std::collections::HashMap;

use serde_json::Value;

use crate::log_follower::{extract_json_values, LogEntry};
use crate::match_payload::{
    Actor, DecisionContext, EventKind, LegalAction, Match, MatchResult, TimelineEvent,
};

const VISIBILITY_HIDDEN: &str = "Visibility_Hidden";
const GAME_STAGE_OVER: &str = "GameStage_GameOver";
const MATCH_STATE_COMPLETE: &str = "MatchState_MatchComplete";
const MATCH_STATE_GAME_COMPLETE: &str = "MatchState_GameComplete";
const MAX_CONTEXT_CARDS: usize = 16;

#[derive(Debug, Clone)]
struct GameObject {
    grp_id: Option<u32>,
    owner_seat: Option<u32>,
    visibility: Option<String>,
    zone_id: Option<u32>,
}

#[derive(Debug, Clone)]
struct Zone {
    zone_type: String,
    owner_seat: Option<u32>,
    instance_ids: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct GreAssembler {
    match_id: Option<String>,
    event_id: Option<String>,
    player_seat: Option<u32>,
    deck_grp_ids: Vec<u32>,
    objects: HashMap<u32, GameObject>,
    zones: HashMap<u32, Zone>,
    life_totals: HashMap<u32, i32>,
    legal: Vec<LegalAction>,
    timeline: Vec<TimelineEvent>,
    turn: Option<u32>,
    phase: Option<String>,
    step: Option<String>,
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
        if self.looks_like_client_message(value) {
            self.handle_client_to_gre(value);
        }
        if let Some(payload) = value.get("payload") {
            if self.looks_like_client_message(payload) {
                self.handle_client_to_gre(payload);
            }
        }
        None
    }

    fn handle_room(&mut self, room: &Value) {
        let info = room.get("gameRoomInfo").unwrap_or(room);
        let state = info.get("stateType").and_then(Value::as_str).unwrap_or("");
        let config = info.get("gameRoomConfig").unwrap_or(info);
        let match_id = string_field(config, &["matchId", "matchID"]);
        let event_id = string_field(config, &["eventId", "eventID"]);

        let event_id = event_id.or_else(|| event_id_from_players(config));
        if state.contains("Playing") {
            if let Some(id) = match_id {
                let is_new_match = self
                    .match_id
                    .as_deref()
                    .is_some_and(|current| current != id.as_str());
                if is_new_match {
                    *self = Self::default();
                }
                self.match_id = Some(id);
                if self.event_id.is_none() {
                    self.event_id = event_id;
                }
                self.in_match = true;
            }
        } else if self.event_id.is_none() {
            self.event_id = event_id;
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
                "GREMessageType_ActionsAvailableReq" => self.handle_actions_available(message),
                "GREMessageType_DeclareAttackersReq" => self.handle_declare_attackers_req(message),
                "GREMessageType_DeclareBlockersReq" => self.handle_declare_blockers_req(message),
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
            if let Some(step) = turn.get("step").and_then(Value::as_str) {
                self.step = Some(step.to_string());
            }
        }

        self.merge_players(gsm.get("players"));
        self.merge_zones(gsm.get("zones"));
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
                zone_id: None,
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
            if let Some(zone_id) = object.get("zoneId").and_then(json_u32) {
                existing.zone_id = Some(zone_id);
            }
        }
    }

    fn merge_zones(&mut self, zones: Option<&Value>) {
        let Some(zones) = zones.and_then(Value::as_array) else {
            return;
        };
        for zone in zones {
            let Some(zone_id) = zone.get("zoneId").and_then(json_u32) else {
                continue;
            };
            let zone_type = zone
                .get("type")
                .or_else(|| zone.get("zoneType"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let owner_seat = zone.get("ownerSeatId").and_then(json_u32);
            let instance_ids = zone
                .get("objectInstanceIds")
                .and_then(Value::as_array)
                .map(|ids| ids.iter().filter_map(json_u32).collect())
                .unwrap_or_default();
            self.zones.insert(
                zone_id,
                Zone {
                    zone_type,
                    owner_seat,
                    instance_ids,
                },
            );
        }
    }

    fn merge_players(&mut self, players: Option<&Value>) {
        let Some(players) = players.and_then(Value::as_array) else {
            return;
        };
        for player in players {
            let Some(seat) = player
                .get("systemSeatNumber")
                .or_else(|| player.get("systemSeatId"))
                .and_then(json_u32)
            else {
                continue;
            };
            if let Some(life) = player.get("lifeTotal").and_then(json_i32) {
                self.life_totals.insert(seat, life);
            }
        }
    }

    fn looks_like_client_message(&self, value: &Value) -> bool {
        value.get("clientToGREMessage").is_some()
            || value.get("clientToGreMessage").is_some()
            || value
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|name| name.starts_with("ClientMessageType_"))
    }

    fn handle_client_to_gre(&mut self, value: &Value) {
        let message = value
            .get("clientToGREMessage")
            .or_else(|| value.get("clientToGreMessage"))
            .unwrap_or(value);
        let msg_type = message.get("type").and_then(Value::as_str).unwrap_or("");
        match msg_type {
            "ClientMessageType_MulliganResp" => {
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
                let cards = self.zone_cards("ZoneType_Hand", self.player_seat);
                let event = self.event(Actor::Me, kind, cards, note);
                self.timeline.push(event);
            }
            "ClientMessageType_DeclareAttackersResp" => {
                let attackers = message
                    .get("declareAttackersResp")
                    .and_then(|resp| resp.get("selectedAttackers"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|attacker| attacker.get("attackerInstanceId").and_then(json_u32))
                    .collect::<Vec<_>>();
                if attackers.is_empty() {
                    return;
                }
                let cards = self.visible_card_ids(&attackers);
                let event = self.event(Actor::Me, EventKind::Attack, cards, "attack");
                self.timeline.push(event);
            }
            "ClientMessageType_DeclareBlockersResp" => {
                let blockers = message
                    .get("declareBlockersResp")
                    .and_then(|resp| resp.get("selectedBlockers"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|blocker| blocker.get("blockerInstanceId").and_then(json_u32))
                    .collect::<Vec<_>>();
                if blockers.is_empty() {
                    return;
                }
                let cards = self.visible_card_ids(&blockers);
                let event = self.event(Actor::Me, EventKind::Block, cards, "block");
                self.timeline.push(event);
            }
            _ => {}
        }
    }

    fn handle_actions_available(&mut self, message: &Value) {
        if !self.message_is_for_us(message) {
            return;
        }
        let Some(actions) = message
            .get("actionsAvailableReq")
            .and_then(|req| req.get("actions"))
            .and_then(Value::as_array)
        else {
            return;
        };
        self.legal = actions
            .iter()
            .filter_map(legal_action_from)
            .take(MAX_CONTEXT_CARDS)
            .collect();
    }

    fn handle_declare_attackers_req(&mut self, message: &Value) {
        if !self.message_is_for_us(message) {
            return;
        }
        let attackers = message
            .get("declareAttackersReq")
            .and_then(|req| req.get("qualifiedAttackers").or_else(|| req.get("attackers")))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|attacker| attacker.get("attackerInstanceId").and_then(json_u32));
        self.legal = attackers
            .filter_map(|id| {
                Some(LegalAction {
                    action: "attack".into(),
                    grp_id: self.objects.get(&id)?.grp_id,
                })
            })
            .take(MAX_CONTEXT_CARDS)
            .collect();
    }

    fn handle_declare_blockers_req(&mut self, message: &Value) {
        if !self.message_is_for_us(message) {
            return;
        }
        let blockers = message
            .get("declareBlockersReq")
            .and_then(|req| req.get("blockers"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut attacker_ids = Vec::new();
        for blocker in &blockers {
            if let Some(ids) = blocker.get("attackerInstanceIds").and_then(Value::as_array) {
                for id in ids.iter().filter_map(json_u32) {
                    if !attacker_ids.contains(&id) {
                        attacker_ids.push(id);
                    }
                }
            }
        }
        if !attacker_ids.is_empty() {
            let cards = self.visible_card_ids(&attacker_ids);
            let event = self.event(Actor::Opponent, EventKind::Attack, cards, "attack");
            self.timeline.push(event);
        }
        self.legal = blockers
            .iter()
            .filter_map(|blocker| blocker.get("blockerInstanceId").and_then(json_u32))
            .filter_map(|id| {
                Some(LegalAction {
                    action: "block".into(),
                    grp_id: self.objects.get(&id)?.grp_id,
                })
            })
            .take(MAX_CONTEXT_CARDS)
            .collect();
    }

    fn message_is_for_us(&self, message: &Value) -> bool {
        let Some(seat) = self.player_seat else {
            return true;
        };
        let Some(seats) = message.get("systemSeatIds").and_then(Value::as_array) else {
            return true;
        };
        seats.iter().filter_map(json_u32).any(|id| id == seat)
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
                    Some("Draw") => EventKind::Draw,
                    Some("Resolve") => EventKind::Resolve,
                    Some("SBA_Damage") => EventKind::Die,
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
            "AnnotationType_DeclaredBlockers" => (EventKind::Block, "block".into()),
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
        let context = self.decision_context(kind);
        TimelineEvent {
            t_ms,
            turn: self.turn,
            phase: self.phase.clone(),
            step: self.step.clone(),
            actor,
            kind,
            card_ids,
            note: note.to_string(),
            context,
        }
    }

    fn decision_context(&self, kind: EventKind) -> Option<DecisionContext> {
        if !matches!(
            kind,
            EventKind::Cast
                | EventKind::Land
                | EventKind::Attack
                | EventKind::Block
                | EventKind::Keep
                | EventKind::Mulligan
        ) {
            return None;
        }
        let seat = self.player_seat;
        let opp = seat.map(|id| if id == 1 { 2 } else { 1 });
        let context = DecisionContext {
            my_hand: self.zone_cards("ZoneType_Hand", seat),
            my_board: self.battlefield_cards(seat),
            opp_board: self.battlefield_cards(opp),
            opp_hand: self.zone_cards("ZoneType_Hand", opp),
            opp_hand_count: self.zone_size("ZoneType_Hand", opp),
            my_life: seat.and_then(|id| self.life_totals.get(&id).copied()),
            opp_life: opp.and_then(|id| self.life_totals.get(&id).copied()),
            legal: self.legal.clone(),
        };
        if context.is_empty() {
            None
        } else {
            Some(context)
        }
    }

    fn zone_size(&self, zone_type: &str, owner: Option<u32>) -> Option<u32> {
        let owner = owner?;
        let sizes = self
            .zones
            .values()
            .filter(|zone| zone.zone_type == zone_type)
            .filter(|zone| zone.owner_seat == Some(owner))
            .map(|zone| zone.instance_ids.len())
            .collect::<Vec<_>>();
        if sizes.is_empty() {
            return None;
        }
        Some(sizes.iter().sum::<usize>() as u32)
    }

    fn zone_cards(&self, zone_type: &str, owner: Option<u32>) -> Vec<u32> {
        let ids = self
            .zones
            .values()
            .filter(|zone| zone.zone_type == zone_type)
            .filter(|zone| owner.is_none() || zone.owner_seat == owner)
            .flat_map(|zone| zone.instance_ids.iter().copied())
            .collect::<Vec<_>>();
        self.visible_card_ids(&ids)
            .into_iter()
            .take(MAX_CONTEXT_CARDS)
            .collect()
    }

    fn battlefield_cards(&self, owner: Option<u32>) -> Vec<u32> {
        let battlefield_zone_ids = self
            .zones
            .iter()
            .filter(|(_, zone)| zone.zone_type == "ZoneType_Battlefield")
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        let mut instance_ids = self
            .zones
            .values()
            .filter(|zone| zone.zone_type == "ZoneType_Battlefield")
            .flat_map(|zone| zone.instance_ids.iter().copied())
            .collect::<Vec<_>>();
        if instance_ids.is_empty() {
            instance_ids = self
                .objects
                .iter()
                .filter(|(_, object)| {
                    object
                        .zone_id
                        .is_some_and(|zone| battlefield_zone_ids.contains(&zone))
                })
                .map(|(id, _)| *id)
                .collect();
        }
        instance_ids.retain(|id| {
            owner.is_none()
                || self
                    .objects
                    .get(id)
                    .is_some_and(|object| object.owner_seat == owner)
        });
        self.visible_card_ids(&instance_ids)
            .into_iter()
            .take(MAX_CONTEXT_CARDS)
            .collect()
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

fn event_id_from_players(config: &Value) -> Option<String> {
    config
        .get("reservedPlayers")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|player| string_field(player, &["eventId", "eventID"]))
}

fn legal_action_from(action: &Value) -> Option<LegalAction> {
    let raw = action.get("actionType").and_then(Value::as_str)?;
    let name = raw.strip_prefix("ActionType_").unwrap_or(raw).to_ascii_lowercase();
    if name == "pass" {
        return Some(LegalAction {
            action: name,
            grp_id: None,
        });
    }
    Some(LegalAction {
        action: name,
        grp_id: action.get("grpId").and_then(json_u32),
    })
}

fn json_i32(value: &Value) -> Option<i32> {
    value
        .as_i64()
        .map(|n| n as i32)
        .or_else(|| value.as_u64().map(|n| n as i32))
        .or_else(|| value.as_str()?.parse().ok())
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
    fn connect_resp_before_playing_room_keeps_deck() {
        let log = r#"[UnityCrossThreadLogger]
{"greToClientEvent":{"greToClientMessages":[{"type":"GREMessageType_ConnectResp","systemSeatIds":[2],"connectResp":{"deckMessage":{"deckCards":[94111,93877,68398]}}}]}}
[UnityCrossThreadLogger]
{"matchGameRoomStateChangedEvent":{"gameRoomInfo":{"stateType":"MatchGameRoomStateType_Playing","gameRoomConfig":{"matchId":"match-connect-first","reservedPlayers":[{"eventId":"DualColorPrecons"}]}}}}
[UnityCrossThreadLogger]
{"greToClientEvent":{"greToClientMessages":[{"type":"GREMessageType_GameStateMessage","gameStateMessage":{"gameInfo":{"matchID":"match-connect-first","stage":"GameStage_GameOver","matchState":"MatchState_GameComplete","results":[{"winningTeamId":2}]}}}]}}
"#;
        let assembled = assemble(log).expect("game over should emit a match");
        assert_eq!(assembled.client_match_id, "match-connect-first");
        assert_eq!(assembled.event_id, "DualColorPrecons");
        assert_eq!(assembled.player_seat, 2);
        assert_eq!(assembled.deck_grp_ids, vec![94111, 93877, 68398]);
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

    #[test]
    fn captures_hands_life_legal_actions_and_live_combat() {
        let log = include_str!("../fixtures/decision_context.log");
        let assembled = assemble(log).expect("context fixture should complete");
        assert_eq!(assembled.event_id, "DualColorPrecons");
        assert_eq!(assembled.format, Format::Constructed);
        assert_eq!(assembled.player_seat, 2);
        assert_eq!(assembled.deck_grp_ids, vec![94111, 93877, 68398]);
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Keep && event.card_ids.contains(&94111))
        );
        let land = assembled
            .timeline
            .iter()
            .find(|event| event.kind == EventKind::Land)
            .expect("land");
        let context = land.context.as_ref().expect("land context");
        assert_eq!(context.opp_hand_count, Some(3));
        assert_eq!(context.opp_hand, vec![94051]);
        assert!(context.my_hand.contains(&94111));
        assert!(context.my_board.contains(&93645));
        assert!(context.opp_board.contains(&94051));
        assert_eq!(context.my_life, Some(18));
        assert_eq!(context.opp_life, Some(20));
        assert!(
            context
                .legal
                .iter()
                .any(|action| action.action == "cast" && action.grp_id == Some(93877))
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Attack && event.actor == Actor::Me)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Attack && event.actor == Actor::Opponent)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Block)
        );
        assert!(
            assembled
                .timeline
                .iter()
                .any(|event| event.kind == EventKind::Draw)
        );
    }

    #[test]
    #[ignore]
    fn assemble_player_log_to_json() {
        let log_path = std::env::var("PLAYER_LOG").expect("PLAYER_LOG");
        let out_path = std::env::var("MATCH_JSON").expect("MATCH_JSON");
        let match_id = std::env::var("CLIENT_MATCH_ID").ok();
        let log = std::fs::read_to_string(&log_path).expect("read player log");
        let (entries, _) = split_entries(&log);
        let mut assembler = GreAssembler::new();
        let mut last = None;
        let mut found = None;
        for entry in entries {
            if let Some(finished) = assembler.push(&entry) {
                if match_id.as_deref() == Some(finished.client_match_id.as_str()) {
                    found = Some(finished.clone());
                }
                last = Some(finished);
            }
        }
        let assembled = found.or(last).expect("no completed match in log");
        std::fs::write(out_path, serde_json::to_vec_pretty(&assembled).unwrap())
            .expect("write match json");
    }
}
