use serde::{Deserialize, Serialize};

// === Auth ===

#[derive(Serialize)]
pub struct RegisterRequest {
    pub name: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Deserialize, Serialize)]
pub struct RegisterResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "createdAt")]
    pub created_at: u64,
}

#[derive(Deserialize)]
pub struct ChallengeResponse {
    pub challenge: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
}

#[derive(Serialize)]
pub struct LoginRequest {
    #[serde(rename = "agentId")]
    pub agent_id: String,
    pub challenge: String,
    pub signature: String,
}

#[derive(Deserialize, Serialize)]
pub struct LoginResponse {
    pub token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
}

// === Economy ===

#[derive(Deserialize, Serialize)]
pub struct WalletResponse {
    pub balance: f64,
    #[serde(rename = "totalEarned")]
    pub total_earned: f64,
    #[serde(rename = "totalSpent")]
    pub total_spent: f64,
}

#[derive(Deserialize, Serialize)]
pub struct Transaction {
    pub id: String,
    #[serde(rename = "agentId")]
    pub agent_id: String,
    #[serde(rename = "type")]
    pub tx_type: String,
    pub amount: f64,
    #[serde(rename = "balanceAfter")]
    pub balance_after: f64,
    pub timestamp: u64,
    #[serde(rename = "gameId")]
    pub game_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// === Game ===

#[derive(Deserialize, Serialize, Debug)]
pub struct ActiveGameResponse {
    #[serde(rename = "gameId")]
    pub game_id: Option<String>,
    pub state: Option<String>,
    #[serde(rename = "gameType")]
    pub game_type: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GameInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "minPlayers")]
    pub min_players: u32,
    #[serde(rename = "maxPlayers")]
    pub max_players: u32,
    #[serde(rename = "defaultConfig", skip_serializing_if = "Option::is_none")]
    pub default_config: Option<serde_json::Value>,
}

/// Platform returns { gameId, state, visibleState } from GET /api/game/:id
#[derive(Deserialize, Serialize, Debug)]
pub struct GameStatusResponse {
    #[serde(rename = "gameId")]
    pub game_id: String,
    pub state: String,
    #[serde(rename = "visibleState")]
    pub visible_state: serde_json::Value,
}

/// Phase info extracted from visibleState.currentPhase
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PhaseInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub phase_type: String,
    pub name: String,
    #[serde(rename = "usesCommitReveal")]
    pub uses_commit_reveal: bool,
    pub timeout: u64,
}

#[derive(Serialize)]
pub struct CommitPayload {
    #[serde(rename = "gameId")]
    pub game_id: String,
    #[serde(rename = "phaseId")]
    pub phase_id: String,
    pub hash: String,
    pub signature: String,
    pub timestamp: u64,
}

#[derive(Serialize)]
pub struct RevealPayload {
    #[serde(rename = "gameId")]
    pub game_id: String,
    #[serde(rename = "phaseId")]
    pub phase_id: String,
    pub data: serde_json::Value,
    pub nonce: String,
    pub signature: String,
    pub timestamp: u64,
}

#[derive(Serialize)]
pub struct ActionPayload {
    #[serde(rename = "gameId")]
    pub game_id: String,
    #[serde(rename = "phaseId")]
    pub phase_id: String,
    #[serde(rename = "actionType")]
    pub action_type: String,
    pub data: serde_json::Value,
    pub signature: String,
    pub timestamp: u64,
}

#[derive(Serialize)]
pub struct EnqueueRequest {
    #[serde(rename = "gameType")]
    pub game_type: String,
}

// === SSE Events ===

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MatchEvent {
    #[serde(rename = "match_found")]
    MatchFound {
        #[serde(rename = "gameId")]
        game_id: String,
        opponent: String,
        #[serde(rename = "gameType")]
        game_type: String,
    },
    #[serde(rename = "match_timeout")]
    MatchTimeout,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type")]
pub enum GameEvent {
    #[serde(rename = "game_start")]
    GameStart {
        players: Vec<String>,
        config: serde_json::Value,
    },
    #[serde(rename = "phase_start")]
    PhaseStart { phase: PhaseInfo },
    #[serde(rename = "all_committed")]
    AllCommitted { phase: String },
    #[serde(rename = "phase_result")]
    PhaseResult {
        phase: String,
        result: serde_json::Value,
    },
    #[serde(rename = "your_turn")]
    YourTurn { phase: String },
    #[serde(rename = "opponent_acted")]
    OpponentActed { phase: String },
    #[serde(rename = "game_end")]
    GameEnd {
        winner: Option<String>,
        reason: String,
    },
    #[serde(rename = "opponent_disconnected")]
    OpponentDisconnected {
        #[serde(rename = "gracePeriod")]
        grace_period: u64,
    },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

// === Audit ===

#[derive(Deserialize, Serialize)]
pub struct SignedLogEntry {
    pub seq: u64,
    pub timestamp: u64,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(rename = "agentId", skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    pub data: serde_json::Value,
    pub signature: String,
    #[serde(rename = "platformSignature")]
    pub platform_signature: String,
    #[serde(rename = "prevHash")]
    pub prev_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_event_found_deserialize() {
        let json = r#"{"type":"match_found","gameId":"g1","opponent":"agent2","gameType":"rps"}"#;
        let event: MatchEvent = serde_json::from_str(json).unwrap();
        match event {
            MatchEvent::MatchFound {
                game_id,
                opponent,
                game_type,
            } => {
                assert_eq!(game_id, "g1");
                assert_eq!(opponent, "agent2");
                assert_eq!(game_type, "rps");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn match_event_timeout_deserialize() {
        let json = r#"{"type":"match_timeout"}"#;
        let event: MatchEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, MatchEvent::MatchTimeout));
    }

    #[test]
    fn game_event_all_committed_deserialize() {
        let json = r#"{"type":"all_committed","phase":"action"}"#;
        let event: GameEvent = serde_json::from_str(json).unwrap();
        match event {
            GameEvent::AllCommitted { phase } => assert_eq!(phase, "action"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn game_event_phase_result_deserialize() {
        let json = r#"{"type":"phase_result","phase":"action","result":{"winner":"a1"}}"#;
        let event: GameEvent = serde_json::from_str(json).unwrap();
        match event {
            GameEvent::PhaseResult { phase, result } => {
                assert_eq!(phase, "action");
                assert_eq!(result["winner"], "a1");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn game_event_game_end_deserialize() {
        let json = r#"{"type":"game_end","winner":"a1","reason":"normal"}"#;
        let event: GameEvent = serde_json::from_str(json).unwrap();
        match event {
            GameEvent::GameEnd { winner, reason } => {
                assert_eq!(winner, Some("a1".to_string()));
                assert_eq!(reason, "normal");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn game_event_game_end_draw_deserialize() {
        let json = r#"{"type":"game_end","winner":null,"reason":"normal"}"#;
        let event: GameEvent = serde_json::from_str(json).unwrap();
        match event {
            GameEvent::GameEnd { winner, .. } => assert!(winner.is_none()),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn commit_payload_serializes_camel_case() {
        let payload = CommitPayload {
            game_id: "g1".into(),
            phase_id: "p1".into(),
            hash: "abc".into(),
            signature: "sig".into(),
            timestamp: 123,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert!(json.get("gameId").is_some());
        assert!(json.get("phaseId").is_some());
        assert!(json.get("game_id").is_none());
    }

    #[test]
    fn action_payload_has_action_type() {
        let payload = ActionPayload {
            game_id: "g1".into(),
            phase_id: "p1".into(),
            action_type: "bid".into(),
            data: serde_json::json!({}),
            signature: "sig".into(),
            timestamp: 123,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["actionType"], "bid");
    }

    #[test]
    fn reveal_payload_has_timestamp() {
        let payload = RevealPayload {
            game_id: "g1".into(),
            phase_id: "p1".into(),
            data: serde_json::json!({}),
            nonce: "n".into(),
            signature: "sig".into(),
            timestamp: 999,
        };
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["timestamp"], 999);
    }

    #[test]
    fn game_status_response_deserialize() {
        let json = r#"{"gameId":"g1","state":"active","visibleState":{"currentPhase":{"id":"p1","type":"simultaneous","name":"action","usesCommitReveal":true,"timeout":30000}}}"#;
        let resp: GameStatusResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.game_id, "g1");
        assert_eq!(resp.state, "active");
    }

    #[test]
    fn register_response_uses_id_not_agent_id() {
        let json = r#"{"id":"a1","name":"bot","publicKey":"pk","createdAt":123}"#;
        let resp: RegisterResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "a1");
    }

    #[test]
    fn login_response_expires_at_is_number() {
        let json = r#"{"token":"tok","expiresAt":9999999999}"#;
        let resp: LoginResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.expires_at, 9999999999);
    }

    #[test]
    fn enqueue_request_serializes_camel_case() {
        let req = EnqueueRequest {
            game_type: "rps".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["gameType"], "rps");
        assert!(json.get("game_type").is_none());
    }

    #[test]
    fn register_request_serializes_camel_case() {
        let req = RegisterRequest {
            name: "bot".into(),
            public_key: "abc".into(),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["publicKey"], "abc");
    }

    #[test]
    fn wallet_response_deserialize() {
        let json = r#"{"balance":100.5,"totalEarned":200.0,"totalSpent":99.5}"#;
        let w: WalletResponse = serde_json::from_str(json).unwrap();
        assert_eq!(w.balance, 100.5);
        assert_eq!(w.total_earned, 200.0);
    }
}
