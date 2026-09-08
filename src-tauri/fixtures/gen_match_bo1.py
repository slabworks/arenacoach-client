#!/usr/bin/env python3
"""Write a redacted-but-complete Constructed Bo1 Player.log slice."""

from __future__ import annotations

import json
from pathlib import Path

OUT = Path(__file__).with_name("match_bo1.log")

DECK = (
    [68398] * 4
    + [73951] * 8
    + [84512] * 4
    + [69420] * 4
    + [70242] * 4
    + [71890] * 4
    + [73301] * 4
    + [75110] * 2
    + [76001] * 4
    + [77112] * 4
    + [78220] * 4
    + [79005] * 4
    + [80111] * 4
    + [81222] * 4
    + [82333] * 2
)

ann_id = 0


def next_ann() -> int:
    global ann_id
    ann_id += 1
    return ann_id


def kv_int(key: str, value: int) -> dict:
    return {"key": key, "type": "KeyValuePairValueType_int32", "valueInt32": [value]}


def kv_str(key: str, value: str) -> dict:
    return {"key": key, "type": "KeyValuePairValueType_string", "valueString": [value]}


def game_object(instance_id: int, grp_id: int, owner: int, visibility: str = "Visibility_Public") -> dict:
    return {
        "instanceId": instance_id,
        "grpId": grp_id,
        "ownerSeatId": owner,
        "controllerSeatId": owner,
        "visibility": visibility,
    }


def annotation(types: list[str], affector: int, affected: list[int], details: list[dict]) -> dict:
    return {
        "id": next_ann(),
        "affectorId": affector,
        "affectedIds": affected,
        "type": types,
        "details": details,
    }


def zone_transfer(affector: int, instance_id: int, category: str, src: int = 31, dest: int = 32) -> dict:
    return annotation(
        ["AnnotationType_ZoneTransfer"],
        affector,
        [instance_id],
        [kv_int("zone_src", src), kv_int("zone_dest", dest), kv_str("category", category)],
    )


def life(seat: int, amount: int) -> dict:
    return annotation(["AnnotationType_ModifiedLife"], seat, [seat], [kv_int("life", amount)])


def damage(affector: int, amount: int, target: int) -> dict:
    return annotation(
        ["AnnotationType_DamageDealt"],
        affector,
        [target],
        [kv_int("damage", amount)],
    )


def attack(seat: int, attackers: list[int]) -> dict:
    return annotation(["AnnotationType_DeclaredAttackers"], seat, attackers, [])


def phase_end(seat: int) -> dict:
    return annotation(["AnnotationType_PhaseOrStepModified"], seat, [seat], [])


def gsm(turn: int, phase: str, active: int, annotations: list[dict] | None = None, extra: dict | None = None) -> dict:
    body: dict = {
        "turnInfo": {"turnNumber": turn, "phase": phase, "activePlayer": active},
        "gameInfo": {
            "matchID": "match-fixture-001",
            "stage": "GameStage_Play",
            "matchState": "MatchState_GameInProgress",
        },
    }
    if annotations:
        body["annotations"] = annotations
    if extra:
        body.update(extra)
    return {
        "type": "GREMessageType_GameStateMessage",
        "systemSeatIds": [1],
        "gameStateMessage": body,
    }


def queued(turn: int, phase: str, active: int, annotations: list[dict]) -> dict:
    message = gsm(turn, phase, active, annotations)
    message["type"] = "GREMessageType_QueuedGameStateMessage"
    message["queuedGameStateMessage"] = message.pop("gameStateMessage")
    return message


def entry(stamp: str, payload: dict) -> str:
    return f"[UnityCrossThreadLogger]{stamp}\n{json.dumps(payload, indent=2)}\n"


chunks: list[str] = []

chunks.append(
    entry(
        "9/8/2026 5:00:00 PM",
        {
            "matchGameRoomStateChangedEvent": {
                "gameRoomInfo": {
                    "stateType": "MatchGameRoomStateType_Playing",
                    "gameRoomConfig": {
                        "matchId": "match-fixture-001",
                        "eventId": "Constructed_BestOf1",
                        "sessionId": "session-REDACTED",
                        "reservedPlayers": [
                            {
                                "systemSeatId": 1,
                                "userId": "REDACTED_USER",
                                "playerName": "Alice#11111",
                                "screenName": "Alice#11111",
                                "teamId": 1,
                            },
                            {
                                "systemSeatId": 2,
                                "userId": "REDACTED_OPP",
                                "playerName": "Opponent",
                                "teamId": 2,
                            },
                        ],
                    },
                }
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:00:01 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    {
                        "type": "GREMessageType_ConnectResp",
                        "systemSeatIds": [1],
                        "connectResp": {
                            "status": "ConnectionStatus_Success",
                            "deckMessage": {"deckCards": DECK},
                        },
                    }
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:00:02 PM",
        {
            "clientToGREMessage": {
                "type": "ClientMessageType_MulliganResp",
                "systemSeatId": 1,
                "mulliganResp": {"decision": "MulliganOption_Mulligan"},
            }
        },
    )
)
chunks.append(
    entry(
        "9/8/2026 5:00:03 PM",
        {
            "clientToGREMessage": {
                "type": "ClientMessageType_MulliganResp",
                "systemSeatId": 1,
                "mulliganResp": {"decision": "MulliganOption_AcceptHand"},
            }
        },
    )
)

opening_objects = [
    game_object(101, 68398, 1, "Visibility_Private"),
    game_object(102, 84512, 1, "Visibility_Private"),
    game_object(103, 70242, 1, "Visibility_Private"),
    game_object(104, 69420, 1, "Visibility_Private"),
    game_object(105, 76001, 1, "Visibility_Private"),
    game_object(106, 73301, 1, "Visibility_Private"),
    game_object(107, 78220, 1, "Visibility_Private"),
    game_object(108, 79005, 1, "Visibility_Private"),
    game_object(201, 73951, 2, "Visibility_Public"),
    game_object(202, 84512, 2, "Visibility_Public"),
    game_object(203, 71890, 2, "Visibility_Public"),
    game_object(204, 77112, 2, "Visibility_Public"),
    game_object(205, 75110, 2, "Visibility_Public"),
    game_object(206, 80111, 2, "Visibility_Public"),
    game_object(999, 12, 1, "Visibility_Private"),
]

chunks.append(
    entry(
        "9/8/2026 5:00:04 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(
                        1,
                        "Phase_Beginning",
                        1,
                        extra={
                            "gameInfo": {
                                "matchID": "match-fixture-001",
                                "stage": "GameStage_Start",
                                "matchState": "MatchState_GameInProgress",
                            },
                            "gameObjects": opening_objects,
                            "zones": [
                                {
                                    "zoneId": 31,
                                    "type": "ZoneType_Hand",
                                    "ownerSeatId": 1,
                                    "objectInstanceIds": [101, 102, 103, 104, 105, 106, 107],
                                }
                            ],
                        },
                    ),
                    gsm(1, "Phase_Main1", 1, [zone_transfer(1, 101, "PlayLand")]),
                    gsm(1, "Phase_Ending", 1, [phase_end(1)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:00:12 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(1, "Phase_Main1", 2, [zone_transfer(2, 201, "PlayLand")]),
                    gsm(1, "Phase_Ending", 2, [phase_end(2)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:00:24 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(2, "Phase_Main1", 1, [zone_transfer(1, 102, "PlayLand")]),
                    queued(2, "Phase_Main1", 1, [zone_transfer(1, 103, "CastSpell")]),
                    gsm(2, "Phase_Ending", 1, [phase_end(1)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:00:36 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(2, "Phase_Main1", 2, [zone_transfer(2, 202, "PlayLand")]),
                    gsm(2, "Phase_Main1", 2, [zone_transfer(2, 203, "CastSpell")]),
                    gsm(2, "Phase_Ending", 2, [phase_end(2)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:00:48 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(
                        3,
                        "Phase_Main1",
                        1,
                        [
                            zone_transfer(1, 104, "PlayLand"),
                            zone_transfer(1, 105, "CastSpell"),
                        ],
                        extra={"diffDeletedInstanceIds": [999]},
                    ),
                    gsm(3, "Phase_Ending", 1, [phase_end(1)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:01:00 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(3, "Phase_Main1", 2, [zone_transfer(2, 204, "CastSpell")]),
                    gsm(3, "Phase_Ending", 2, [phase_end(2)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:01:14 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(4, "Phase_Combat", 1, [attack(1, [103])]),
                    gsm(4, "Phase_Combat", 1, [damage(103, 2, 2), life(2, -2)]),
                    gsm(4, "Phase_Ending", 1, [phase_end(1)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:01:28 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(4, "Phase_Main1", 2, [zone_transfer(2, 206, "CastSpell")]),
                    gsm(4, "Phase_Combat", 2, [attack(2, [203])]),
                    gsm(4, "Phase_Combat", 2, [damage(203, 3, 1), life(1, -3)]),
                    gsm(4, "Phase_Ending", 2, [phase_end(2)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:01:42 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(5, "Phase_Main1", 1, [zone_transfer(1, 106, "CastSpell")]),
                    gsm(5, "Phase_Combat", 1, [attack(1, [103, 106])]),
                    gsm(5, "Phase_Combat", 1, [damage(103, 2, 2), damage(106, 3, 2), life(2, -5)]),
                    gsm(5, "Phase_Ending", 1, [phase_end(1)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:01:56 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(5, "Phase_Main1", 2, [zone_transfer(2, 205, "CastSpell")]),
                    gsm(5, "Phase_Ending", 2, [phase_end(2)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:02:10 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(6, "Phase_Main1", 1, [zone_transfer(1, 107, "CastSpell")]),
                    gsm(6, "Phase_Combat", 1, [attack(1, [103, 106, 107])]),
                    gsm(
                        6,
                        "Phase_Combat",
                        1,
                        [damage(103, 2, 2), damage(106, 3, 2), damage(107, 4, 2), life(2, -9)],
                    ),
                    gsm(6, "Phase_Ending", 1, [phase_end(1)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:02:22 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(
                        6,
                        "Phase_Main1",
                        2,
                        [zone_transfer(2, 203, "Discard", src=31, dest=33)],
                    ),
                    gsm(6, "Phase_Ending", 2, [phase_end(2)]),
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:02:36 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    gsm(7, "Phase_Combat", 1, [attack(1, [103, 106, 107])]),
                    gsm(
                        7,
                        "Phase_Combat",
                        1,
                        [damage(103, 2, 2), damage(106, 3, 2), damage(107, 4, 2), life(2, -9)],
                    ),
                    {
                        "type": "GREMessageType_GameStateMessage",
                        "systemSeatIds": [1],
                        "gameStateMessage": {
                            "turnInfo": {"turnNumber": 7, "phase": "Phase_Ending", "activePlayer": 1},
                            "gameInfo": {
                                "matchID": "match-fixture-001",
                                "stage": "GameStage_GameOver",
                                "matchState": "MatchState_GameComplete",
                                "results": [
                                    {
                                        "scope": "MatchScope_Game",
                                        "result": "ResultType_WinLoss",
                                        "winningTeamId": 1,
                                    }
                                ],
                            },
                        },
                    },
                ]
            }
        },
    )
)

chunks.append(
    entry(
        "9/8/2026 5:02:37 PM",
        {
            "greToClientEvent": {
                "greToClientMessages": [
                    {
                        "type": "GREMessageType_GameStateMessage",
                        "systemSeatIds": [1],
                        "gameStateMessage": {
                            "gameInfo": {
                                "stage": "GameStage_GameOver",
                                "matchState": "MatchState_MatchComplete",
                                "results": [
                                    {
                                        "scope": "MatchScope_Match",
                                        "result": "ResultType_WinLoss",
                                        "winningTeamId": 1,
                                    }
                                ],
                            }
                        },
                    }
                ]
            }
        },
    )
)

OUT.write_text("".join(chunks), encoding="utf-8")
print(f"wrote {OUT} ({OUT.stat().st_size} bytes, deck={len(DECK)})")
