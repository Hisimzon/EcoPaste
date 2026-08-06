use anyhow::Context;
use pinyin::ToPinyin;
use sqlx::SqlitePool;

use crate::core::Result;

const MAX_SCAN_CHARS: usize = 4096;
const MAX_PINYIN_CHARS: usize = 200;
const BACKFILL_BATCH_SIZE: i64 = 100;

#[derive(Debug, Default)]
pub struct PinyinIndexCleanupOutcome {
    pub optimized_items: u64,
    pub removed_bytes: u64,
}

pub fn build_search_index(search_text: Option<&str>, note: Option<&str>) -> (String, String) {
    let mut compact = String::new();
    let mut syllables = String::new();
    let mut initials = String::new();

    for value in [search_text, note].into_iter().flatten() {
        let mut value_compact = String::new();
        let mut value_syllables = String::new();
        let mut value_initials = String::new();
        let mut pinyin_count = 0;

        for character in value.chars().take(MAX_SCAN_CHARS) {
            let Some(pinyin) = character.to_pinyin() else {
                continue;
            };

            if pinyin_count == MAX_PINYIN_CHARS {
                break;
            }

            let plain = pinyin.plain();
            value_compact.push_str(plain);
            if !value_syllables.is_empty() {
                value_syllables.push(' ');
            }
            value_syllables.push_str(plain);
            if let Some(initial) = plain.chars().next() {
                value_initials.push(initial);
            }
            pinyin_count += 1;
        }

        if value_compact.is_empty() {
            continue;
        }

        if !compact.is_empty() {
            compact.push(' ');
            syllables.push(' ');
            initials.push(' ');
        }

        compact.push_str(&value_compact);
        syllables.push_str(&value_syllables);
        initials.push_str(&value_initials);
    }

    let search_pinyin = if syllables.is_empty() {
        compact.clone()
    } else {
        format!("{compact} {syllables}")
    };

    (search_pinyin, initials)
}

pub async fn backfill(pool: &SqlitePool) -> Result<()> {
    let mut last_rowid = 0;

    loop {
        let rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>)>(
            "SELECT rowid, search_text, note FROM clipboard_items
             WHERE rowid > ? AND (search_pinyin IS NULL OR search_pinyin_initials IS NULL)
             ORDER BY rowid LIMIT ?",
        )
        .bind(last_rowid)
        .bind(BACKFILL_BATCH_SIZE)
        .fetch_all(pool)
        .await
        .context("failed to load clipboard items for pinyin backfill")?;

        if rows.is_empty() {
            break;
        }

        last_rowid = rows.last().map(|row| row.0).unwrap_or(last_rowid);
        let indexed_rows = tokio::task::spawn_blocking(move || {
            rows.into_iter()
                .map(|(rowid, search_text, note)| {
                    let index = build_search_index(search_text.as_deref(), note.as_deref());
                    (rowid, search_text, note, index)
                })
                .collect::<Vec<_>>()
        })
        .await
        .context("pinyin backfill worker panicked")?;

        for (rowid, search_text, note, (search_pinyin, search_pinyin_initials)) in indexed_rows {
            sqlx::query(
                "UPDATE clipboard_items
                 SET search_pinyin = ?, search_pinyin_initials = ?
                 WHERE rowid = ? AND search_text IS ? AND note IS ?",
            )
            .bind(search_pinyin)
            .bind(search_pinyin_initials)
            .bind(rowid)
            .bind(search_text)
            .bind(note)
            .execute(pool)
            .await
            .context("failed to update clipboard item pinyin index")?;
        }
    }

    Ok(())
}

/// 将已有备注记录的拼音索引重建为仅包含备注，释放正文拼音占用。
pub async fn compact_noted_item_indexes(pool: &SqlitePool) -> Result<PinyinIndexCleanupOutcome> {
    let mut last_rowid = 0;
    let mut outcome = PinyinIndexCleanupOutcome::default();

    loop {
        let rows = sqlx::query_as::<_, (i64, String, String, String)>(
            "SELECT rowid, note, search_pinyin, search_pinyin_initials
             FROM clipboard_items
             WHERE rowid > ?
               AND note IS NOT NULL
               AND trim(note) <> ''
               AND search_pinyin IS NOT NULL
               AND search_pinyin_initials IS NOT NULL
             ORDER BY rowid LIMIT ?",
        )
        .bind(last_rowid)
        .bind(BACKFILL_BATCH_SIZE)
        .fetch_all(pool)
        .await
        .context("failed to load noted clipboard item pinyin indexes")?;

        if rows.is_empty() {
            break;
        }

        last_rowid = rows.last().map(|row| row.0).unwrap_or(last_rowid);
        let compacted_rows = tokio::task::spawn_blocking(move || {
            rows.into_iter()
                .map(|(rowid, note, current_pinyin, current_initials)| {
                    let (next_pinyin, next_initials) = build_search_index(None, Some(&note));
                    (
                        rowid,
                        note,
                        current_pinyin,
                        current_initials,
                        next_pinyin,
                        next_initials,
                    )
                })
                .collect::<Vec<_>>()
        })
        .await
        .context("pinyin index cleanup worker panicked")?;

        for (
            rowid,
            note,
            current_pinyin,
            current_initials,
            next_pinyin,
            next_initials,
        ) in compacted_rows
        {
            if current_pinyin == next_pinyin && current_initials == next_initials {
                continue;
            }

            let removed_bytes = current_pinyin
                .len()
                .saturating_add(current_initials.len())
                .saturating_sub(next_pinyin.len().saturating_add(next_initials.len()));
            let result = sqlx::query(
                "UPDATE clipboard_items
                 SET search_pinyin = ?, search_pinyin_initials = ?
                 WHERE rowid = ?
                   AND note = ?
                   AND search_pinyin = ?
                   AND search_pinyin_initials = ?",
            )
            .bind(next_pinyin)
            .bind(next_initials)
            .bind(rowid)
            .bind(note)
            .bind(current_pinyin)
            .bind(current_initials)
            .execute(pool)
            .await
            .context("failed to compact clipboard item pinyin index")?;

            if result.rows_affected() == 0 {
                continue;
            }

            outcome.optimized_items += 1;
            outcome.removed_bytes += removed_bytes as u64;
        }
    }

    Ok(outcome)
}

pub fn queue_update(pool: &SqlitePool, id: &str) {
    let pool = pool.clone();
    let id = id.to_owned();

    tokio::spawn(async move {
        let result = update_one(&pool, &id).await;
        if let Err(err) = result {
            log::warn!("failed to update clipboard item pinyin index: {err}");
        }
    });
}

async fn update_one(pool: &SqlitePool, id: &str) -> Result<()> {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT search_text, note FROM clipboard_items WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("failed to load clipboard item for pinyin index")?;

    let Some((search_text, note)) = row else {
        return Ok(());
    };

    let (search_pinyin, search_pinyin_initials) = tokio::task::spawn_blocking({
        let search_text = search_text.clone();
        let note = note.clone();
        move || build_search_index(search_text.as_deref(), note.as_deref())
    })
    .await
    .context("pinyin index worker panicked")?;

    sqlx::query(
        "UPDATE clipboard_items
         SET search_pinyin = ?, search_pinyin_initials = ?
         WHERE id = ? AND search_text IS ? AND note IS ?",
    )
    .bind(search_pinyin)
    .bind(search_pinyin_initials)
    .bind(id)
    .bind(search_text)
    .bind(note)
    .execute(pool)
    .await
    .context("failed to save clipboard item pinyin index")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_search_index;

    #[test]
    fn note_only_index_excludes_body_pinyin() {
        let (full, _) = build_search_index(Some("正文"), Some("备注"));
        let (note_only, initials) = build_search_index(None, Some("备注"));

        assert!(full.contains("zhengwen"));
        assert!(!note_only.contains("zhengwen"));
        assert!(note_only.contains("beizhu"));
        assert_eq!(initials, "bz");
    }
}
