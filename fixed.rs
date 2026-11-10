//! Fixed expenses: payments due every month (subscriptions, rent, phone bill...).
//! The list lives in the setting `expense.fixed`; which ones were paid in which month in
//! `expense.fixedPaid` as `{ "YYYY-MM": { "<fixed id>": <expense id> } }`.
//! Ticking one off records a real expense on its due date; unticking removes that record.
//! The PC page and the phone (through cloud sync) both go through `pay` / `unpay`.

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::expenses::Expense;

pub const KEY_LIST: &str = "expense.fixed";
pub const KEY_PAID: &str = "expense.fixedPaid";
/// Paid marks older than this many months are dropped when saving.
const KEEP_MONTHS: i32 = 24;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fixed {
    pub id: String,
    pub name: String,
    pub amount: f64,
    pub category: String,
    /// Day of the month it is due (1–31; clamped to the month's last day).
    pub day: u32,
}

/// month ("YYYY-MM") -> fixed id -> expense id
pub type Paid = BTreeMap<String, BTreeMap<String, i64>>;

pub fn list(db: &Db) -> Vec<Fixed> {
    db.setting(KEY_LIST)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Vec<Fixed>>(&raw).ok())
        .unwrap_or_default()
}

pub fn paid(db: &Db) -> Paid {
    db.setting(KEY_PAID).ok().flatten().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

fn save_paid(db: &Db, mut paid: Paid) -> Result<(), String> {
    // Keep the recent past only.
    let cutoff = {
        let today = chrono::Local::now().date_naive();
        let months = today.year() * 12 + today.month0() as i32 - KEEP_MONTHS;
        format!("{:04}-{:02}", months.div_euclid(12), months.rem_euclid(12) + 1)
    };
    paid.retain(|month, ids| month.as_str() >= cutoff.as_str() && !ids.is_empty());
    let raw = serde_json::to_string(&paid).map_err(|e| e.to_string())?;
    db.set_setting(KEY_PAID, &raw).map_err(|e| e.to_string())
}

/// The due date of a fixed expense in `month` ("YYYY-MM"), clamped to the month's length.
pub fn due_date(month: &str, day: u32) -> Result<NaiveDate, String> {
    let first = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").map_err(|_| format!("잘못된 달: {month}"))?;
    let next = first.checked_add_months(chrono::Months::new(1)).ok_or("잘못된 달")?;
    let last = next.pred_opt().ok_or("잘못된 달")?.day();
    Ok(first.with_day(day.clamp(1, last)).unwrap_or(first))
}

/// Marks a fixed expense as paid for `month`, recording the expense. Returns the expense id.
pub fn pay(db: &Db, fixed_id: &str, month: &str) -> Result<i64, String> {
    let item = list(db).into_iter().find(|f| f.id == fixed_id).ok_or("고정 지출을 찾을 수 없어요")?;
    let mut paid = paid(db);
    if let Some(existing) = paid.get(month).and_then(|m| m.get(fixed_id)).copied() {
        let still_there = db
            .conn
            .query_row("SELECT 1 FROM expenses WHERE id=?1", [existing], |_| Ok(()))
            .is_ok();
        if still_there {
            return Ok(existing);
        }
    }
    let expense = Expense {
        id: 0,
        amount: item.amount,
        category: item.category.clone(),
        date: due_date(month, item.day)?,
        note: item.name.clone(),
        kind: "expense".into(),
    };
    let id = db.save_expense(&expense).map_err(|e| e.to_string())?;
    paid.entry(month.to_owned()).or_default().insert(fixed_id.to_owned(), id);
    save_paid(db, paid)?;
    Ok(id)
}

/// Undoes `pay`: removes the recorded expense and the mark.
pub fn unpay(db: &Db, fixed_id: &str, month: &str) -> Result<(), String> {
    let mut paid = paid(db);
    if let Some(id) = paid.get_mut(month).and_then(|m| m.remove(fixed_id)) {
        db.delete_expense(id).map_err(|e| e.to_string())?;
    }
    save_paid(db, paid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_dates_fit_the_month() {
        assert_eq!(due_date("2026-02", 31).unwrap(), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
        assert_eq!(due_date("2026-11", 15).unwrap(), NaiveDate::from_ymd_opt(2026, 11, 15).unwrap());
        assert_eq!(due_date("2026-12", 0).unwrap(), NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        assert!(due_date("nope", 1).is_err());
    }
}
