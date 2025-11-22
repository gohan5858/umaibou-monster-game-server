use crate::models::{Character, GameState, InputAction, PlayerInput};
use crate::utils::{add_vector3, multiply_vector3, normalize_vector3};
use chrono::Utc;
use uuid::Uuid;

pub struct GameStateManager {
    pub matching_id: Uuid,
    pub player_a_id: String,
    pub player_b_id: String,
    pub player_a_character: Character,
    pub player_b_character: Character,
}

impl GameStateManager {
    pub fn new(
        matching_id: Uuid,
        player_a_id: String,
        player_b_id: String,
        player_a_character: Character,
        player_b_character: Character,
    ) -> Self {
        Self {
            matching_id,
            player_a_id,
            player_b_id,
            player_a_character,
            player_b_character,
        }
    }

    /// プレイヤー入力を処理
    pub fn process_input(&mut self, input: PlayerInput) {
        let character = if input.player_id == self.player_a_id {
            &mut self.player_a_character
        } else if input.player_id == self.player_b_id {
            &mut self.player_b_character
        } else {
            return; // 不明なプレイヤー
        };

        match input.action {
            InputAction::Move { direction, speed } => {
                let normalized = normalize_vector3(&direction);
                let velocity = multiply_vector3(&normalized, speed * 0.016667); // 1/60秒
                character.position = add_vector3(&character.position, &velocity);
            }
            InputAction::Rotate { rotation } => {
                character.rotation = rotation;
            }
            InputAction::Attack { .. } => {
                // 攻撃処理（ダメージ計算はクライアント側で実施）
                // サーバーは攻撃イベントの検証のみ
            }
        }
    }

    /// プレイヤー状態を直接更新（クライアントからのStateUpdate用）
    pub fn update_state(&mut self, player_id: &str, position: crate::models::Vector3, rotation: crate::models::Vector3) {
        let character = if player_id == self.player_a_id {
            &mut self.player_a_character
        } else if player_id == self.player_b_id {
            &mut self.player_b_character
        } else {
            return; // 不明なプレイヤー
        };

        character.position = position;
        character.rotation = rotation;
    }

    /// ダメージ適用（クライアントからの報告）
    pub fn apply_damage(&mut self, player_id: &str, damage: i32) {
        let character = if player_id == self.player_a_id {
            &mut self.player_a_character
        } else if player_id == self.player_b_id {
            &mut self.player_b_character
        } else {
            return;
        };

        character.hp = (character.hp - damage).max(0);
    }

    /// 現在のゲーム状態を取得（デバッグ用）
    #[allow(dead_code)]
    pub fn get_state(&self) -> GameState {
        GameState {
            matching_id: self.matching_id,
            player_a: self.player_a_character.clone(),
            player_b: self.player_b_character.clone(),
            timestamp: Utc::now(),
        }
    }

    /// 勝者を判定
    pub fn check_winner(&self) -> Option<String> {
        if !self.player_a_character.is_alive() {
            Some(self.player_b_id.clone())
        } else if !self.player_b_character.is_alive() {
            Some(self.player_a_id.clone())
        } else {
            None
        }
    }

    /// ゲームが終了したかチェック
    pub fn is_game_over(&self) -> bool {
        self.check_winner().is_some()
    }
}
