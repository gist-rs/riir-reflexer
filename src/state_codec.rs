//! The wire `state` schema — the measurement contract's state encoding.
//!
//! `DecisionRequest::state` (a JSON string, per the wire) carries THIS
//! object when the questions are tetris game questions. The encoding is
//! exact: `Board::to_strings` → `Board::from_strings` round-trips the cell
//! grid bit-for-bit, pieces cross by their one-letter ids, and the bag
//! remainder is the 7-bag state AFTER `next` was drawn (the same slice
//! `play_game` hands the searcher — depth-3 bag support reads it, so an
//! inexact transport would break decision bit-identity).

use katgpt_tetris::sim::{Board, HEIGHT, Piece, WIDTH};

/// One decision-time game state, in wire form.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    /// Exactly [`HEIGHT`] rows of exactly [`WIDTH`] `#`/`.` chars
    /// (`Board::to_strings` output; row 0 = top).
    pub board: Vec<String>,
    /// Piece id letter (`I O T S Z J L`).
    pub cur: String,
    /// The preview piece id.
    pub next: String,
    /// The held piece id, if any.
    #[serde(default)]
    pub held: Option<String>,
    /// Hold not yet used this drop (the guideline rule). Defaults true —
    /// the `play_game` posture.
    #[serde(default = "default_hold_ready")]
    pub hold_ready: bool,
    /// The 7-bag remainder AFTER `next` was drawn, in draw order. Empty
    /// means a fresh full bag is next.
    #[serde(default)]
    pub bag: Vec<String>,
}

fn default_hold_ready() -> bool {
    true
}

/// The decoded, validated sim-side view of a [`GameState`].
pub struct SimState {
    pub board: Board,
    pub cur: Piece,
    pub next: Piece,
    pub held: Option<Piece>,
    pub hold_ready: bool,
    pub bag: Vec<Piece>,
}

/// Why a state failed to decode (typed, surfaced as `bad_state`).
#[derive(Clone, Debug, PartialEq)]
pub enum StateError {
    Rows(usize),
    RowWidth { row: usize, len: usize },
    RowChars { row: usize },
    Piece(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rows(n) => write!(f, "board must be {HEIGHT} rows, got {n}"),
            Self::RowWidth { row, len } => {
                write!(f, "row {row} must be {WIDTH} chars, got {len}")
            }
            Self::RowChars { row } => write!(f, "row {row} has chars other than '#'/'.'"),
            Self::Piece(s) => write!(f, "unknown piece id {s:?} (want I O T S Z J L)"),
        }
    }
}

pub fn piece_id(p: Piece) -> &'static str {
    p.id()
}

pub fn piece_from_id(s: &str) -> Option<Piece> {
    Piece::ALL.iter().copied().find(|p| p.id() == s)
}

impl GameState {
    /// Encode the sim-side facts in wire form.
    pub fn encode(
        board: &Board,
        cur: Piece,
        next: Piece,
        held: Option<Piece>,
        hold_ready: bool,
        bag: &[Piece],
    ) -> Self {
        Self {
            board: board.to_strings(),
            cur: cur.id().to_string(),
            next: next.id().to_string(),
            held: held.map(|p| p.id().to_string()),
            hold_ready,
            bag: bag.iter().map(|p| p.id().to_string()).collect(),
        }
    }

    /// Validate + decode to the sim-side view. Fail-closed on shape.
    pub fn to_sim(&self) -> Result<SimState, StateError> {
        if self.board.len() != HEIGHT {
            return Err(StateError::Rows(self.board.len()));
        }
        for (r, row) in self.board.iter().enumerate() {
            if row.len() != WIDTH {
                return Err(StateError::RowWidth {
                    row: r,
                    len: row.len(),
                });
            }
            if !row.chars().all(|c| c == '#' || c == '.') {
                return Err(StateError::RowChars { row: r });
            }
        }
        let rows: Vec<&str> = self.board.iter().map(|s| s.as_str()).collect();
        let cur = piece_from_id(&self.cur).ok_or_else(|| StateError::Piece(self.cur.clone()))?;
        let next = piece_from_id(&self.next).ok_or_else(|| StateError::Piece(self.next.clone()))?;
        let held = match &self.held {
            None => None,
            Some(id) => Some(piece_from_id(id).ok_or_else(|| StateError::Piece(id.clone()))?),
        };
        let mut bag = Vec::with_capacity(self.bag.len());
        for id in &self.bag {
            bag.push(piece_from_id(id).ok_or_else(|| StateError::Piece(id.clone()))?);
        }
        Ok(SimState {
            board: Board::from_strings(&rows),
            cur,
            next,
            held,
            hold_ready: self.hold_ready,
            bag,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use katgpt_tetris::lookahead::garbage_board;

    fn roundtrip(board: &Board, cur: Piece, next: Piece, held: Option<Piece>) {
        let s = GameState::encode(board, cur, next, held, true, &[Piece::T, Piece::L]);
        let sim = s.to_sim().unwrap();
        assert_eq!(sim.board.to_strings(), board.to_strings());
        assert_eq!(sim.cur, cur);
        assert_eq!(sim.next, next);
        assert_eq!(sim.held, held);
        assert!(sim.hold_ready);
        assert_eq!(sim.bag, vec![Piece::T, Piece::L]);
    }

    #[test]
    fn roundtrips_empty_and_garbage_boards_all_pieces() {
        let empty = Board::empty();
        for cur in Piece::ALL {
            for next in Piece::ALL {
                roundtrip(&empty, cur, next, None);
            }
        }
        for seed in 0..8u64 {
            let b = garbage_board(seed, 8, 60);
            roundtrip(&b, Piece::T, Piece::I, Some(Piece::Z));
        }
    }

    #[test]
    fn rejects_wrong_shape() {
        let mut s = GameState::encode(&Board::empty(), Piece::I, Piece::O, None, true, &[]);
        s.board = vec![];
        assert_eq!(s.to_sim().err(), Some(StateError::Rows(0)));
        s.board = vec!["..........".to_string(); HEIGHT];
        s.cur = "X".into();
        assert_eq!(s.to_sim().err(), Some(StateError::Piece("X".into())));
        s.cur = "I".into();
        s.board[3] = "###".into();
        assert_eq!(
            s.to_sim().err(),
            Some(StateError::RowWidth { row: 3, len: 3 })
        );
        s.board[3] = "xx..##....".into();
        assert_eq!(s.to_sim().err(), Some(StateError::RowChars { row: 3 }));
    }
}
