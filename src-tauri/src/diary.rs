//! Diary: one page per day with a mood, a layout, the text, a photo and stickers.

use chrono::NaiveDate;
use rusqlite::{OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};

use crate::db::{D_FMT, Db, DbResult, now_ts, parse_d};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DiarySticker {
    pub id: String,
    pub image: String,
    /// Center as a fraction (0–1) of the open book.
    pub x: f64,
    pub y: f64,
    /// Width as a fraction of the open book's width, so stickers scale with the window.
    pub w: f64,
    /// Rotation in degrees.
    pub rot: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub date: NaiveDate,
    /// One of `MOODS`, or empty for none.
    #[serde(default)]
    pub mood: String,
    /// How the two pages are arranged, one of `LAYOUTS`.
    #[serde(default = "DiaryEntry::default_layout")]
    pub layout: String,
    /// Paper of the writing page, one of `PAPERS`.
    #[serde(default = "DiaryEntry::default_paper")]
    pub paper: String,
    /// "Today in one line".
    #[serde(default)]
    pub line: String,
    /// The writing, as sanitized HTML from the editor (keeps highlighter colors).
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub photo: Option<String>,
    #[serde(default)]
    pub stickers: Vec<DiarySticker>,
    #[serde(default)]
    pub updated_at: i64,
}

impl DiaryEntry {
    pub const MOODS: [&'static str; 7] = ["great", "good", "love", "okay", "tired", "sad", "angry"];
    pub const LAYOUTS: [&'static str; 3] = ["classic", "photo", "simple"];
    pub const PAPERS: [&'static str; 4] = ["lined", "grid", "dots", "plain"];

    fn default_layout() -> String {
        "classic".into()
    }

    fn default_paper() -> String {
        "lined".into()
    }

    /// Nothing written or chosen: such a page is not kept.
    pub fn is_blank(&self) -> bool {
        let text = self.body.replace(|c: char| c == '<' || c == '>', " ");
        let has_text = text.split_whitespace().any(|w| !matches!(w, "p" | "/p" | "br" | "p/"));
        self.mood.is_empty() && self.line.trim().is_empty() && !has_text && self.photo.is_none() && self.stickers.is_empty()
    }
}

const COLUMNS: &str = "date, mood, layout, paper, line, body, photo, stickers, updated_at";

fn entry_from_row(r: &Row) -> DbResult<DiaryEntry> {
    let stickers: String = r.get(7)?;
    Ok(DiaryEntry {
        date: parse_d(r.get(0)?),
        mood: r.get(1)?,
        layout: r.get(2)?,
        paper: r.get(3)?,
        line: r.get(4)?,
        body: r.get(5)?,
        photo: r.get(6)?,
        stickers: serde_json::from_str(&stickers).unwrap_or_default(),
        updated_at: r.get(8)?,
    })
}

impl Db {
    /// Diary pages dated within `from..=to`, oldest first.
    pub fn diary_between(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<DiaryEntry>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLUMNS} FROM diary WHERE date >= ?1 AND date <= ?2 ORDER BY date"))?;
        st.query_map([from.format(D_FMT).to_string(), to.format(D_FMT).to_string()], entry_from_row)?.collect()
    }

    /// Saves the page for its day (replacing it). A blank page is removed instead.
    pub fn save_diary_entry(&self, e: &DiaryEntry) -> DbResult<()> {
        let date = e.date.format(D_FMT).to_string();
        if e.is_blank() {
            self.conn.execute("DELETE FROM diary WHERE date=?1", [date])?;
            return Ok(());
        }
        let stickers = serde_json::to_string(&e.stickers).unwrap_or_else(|_| "[]".into());
        self.conn.execute(
            "INSERT INTO diary(date, mood, layout, paper, line, body, photo, stickers, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(date) DO UPDATE SET mood=?2, layout=?3, paper=?4, line=?5, body=?6, photo=?7, stickers=?8, updated_at=?9",
            params![date, e.mood, e.layout, e.paper, e.line, e.body, e.photo, stickers, now_ts()],
        )?;
        Ok(())
    }

    /// Deletes a day's page and returns it, so the deletion can be undone.
    pub fn delete_diary_entry(&self, date: NaiveDate) -> DbResult<Option<DiaryEntry>> {
        let date = date.format(D_FMT).to_string();
        let stored =
            self.conn.query_row(&format!("SELECT {COLUMNS} FROM diary WHERE date=?1"), [&date], entry_from_row).optional()?;
        self.conn.execute("DELETE FROM diary WHERE date=?1", [&date])?;
        Ok(stored)
    }

    /// Photo and sticker files used by diary pages (so image cleanup keeps them).
    pub fn diary_images(&self) -> DbResult<Vec<String>> {
        let mut st = self.conn.prepare("SELECT photo, stickers FROM diary")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (photo, stickers) = row?;
            out.extend(photo);
            let stickers: Vec<DiarySticker> = serde_json::from_str(&stickers).unwrap_or_default();
            out.extend(stickers.into_iter().map(|s| s.image));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::tests::{d, temp_db};

    fn page(date: &str) -> DiaryEntry {
        DiaryEntry {
            date: d(date),
            mood: "good".into(),
            layout: "classic".into(),
            paper: "lined".into(),
            line: String::new(),
            body: "<p>Went to the sea.</p>".into(),
            photo: None,
            stickers: vec![],
            updated_at: 0,
        }
    }

    #[test]
    fn one_page_per_day_and_blank_pages_are_not_kept() {
        let (db, path) = temp_db();
        db.save_diary_entry(&page("2026-10-24")).unwrap();
        db.save_diary_entry(&DiaryEntry { mood: "great".into(), ..page("2026-10-24") }).unwrap();
        let sticker = DiarySticker { id: "s1".into(), image: "a-1.png".into(), x: 0.5, y: 0.5, w: 0.1, rot: 0.0 };
        db.save_diary_entry(&DiaryEntry { stickers: vec![sticker.clone()], ..page("2026-10-25") }).unwrap();

        let week = db.diary_between(d("2026-10-19"), d("2026-10-25")).unwrap();
        assert_eq!(week.len(), 2);
        assert_eq!(week[0].mood, "great");
        assert_eq!(week[1].stickers, vec![sticker]);
        assert_eq!(db.diary_images().unwrap(), ["a-1.png"]);

        // Clearing everything removes the page.
        let blank = DiaryEntry { mood: String::new(), body: "<p></p>".into(), ..page("2026-10-24") };
        assert!(blank.is_blank());
        db.save_diary_entry(&blank).unwrap();
        assert_eq!(db.diary_between(d("2026-10-01"), d("2026-10-31")).unwrap().len(), 1);

        assert!(db.delete_diary_entry(d("2026-10-25")).unwrap().is_some());
        assert!(db.diary_between(d("2026-10-01"), d("2026-10-31")).unwrap().is_empty());
        let _ = std::fs::remove_file(path);
    }
}
