//! Sticky notes: short one-line notes pinned on a board (Stickies page) and listed on the phone.
//! New notes go first; the order can be changed by dragging.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::{Db, DbResult, now_ts};

/// Longest note kept, in characters.
pub const MAX_LEN: usize = 500;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sticky {
    pub id: i64,
    pub text: String,
    pub color: u32,
    #[serde(default)]
    pub created_at: i64,
}

/// Trims the text and caps its length; `None` when nothing is left.
pub fn clean(text: &str) -> Option<String> {
    let t = text.trim();
    (!t.is_empty()).then(|| t.chars().take(MAX_LEN).collect())
}

impl Db {
    /// All notes in board order.
    pub fn stickies(&self) -> DbResult<Vec<Sticky>> {
        let mut st = self.conn.prepare("SELECT id, text, color, created_at FROM stickies ORDER BY position, id DESC")?;
        let rows = st.query_map([], |r| Ok(Sticky { id: r.get(0)?, text: r.get(1)?, color: r.get(2)?, created_at: r.get(3)? }))?;
        rows.collect()
    }

    /// Adds a note at the front of the board.
    pub fn add_sticky(&self, text: &str, color: u32) -> DbResult<i64> {
        let first: i64 = self.conn.query_row("SELECT COALESCE(MIN(position), 0) FROM stickies", [], |r| r.get(0))?;
        self.conn.execute(
            "INSERT INTO stickies(text, color, position, created_at) VALUES(?1, ?2, ?3, ?4)",
            params![text, color, first - 1, now_ts()],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_sticky(&self, id: i64, text: &str, color: u32) -> DbResult<()> {
        self.conn.execute("UPDATE stickies SET text=?1, color=?2 WHERE id=?3", params![text, color, id])?;
        Ok(())
    }

    pub fn delete_sticky(&self, id: i64) -> DbResult<()> {
        self.conn.execute("DELETE FROM stickies WHERE id=?1", [id])?;
        Ok(())
    }

    /// Saves the board order; `ids` lists it first to last.
    pub fn reorder_stickies(&self, ids: &[i64]) -> DbResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        for (position, id) in ids.iter().enumerate() {
            tx.execute("UPDATE stickies SET position=?1 WHERE id=?2", params![position as i64, id])?;
        }
        tx.commit()
    }
}
