use anyhow::Context;
use pinyin::ToPinyin;
use sqlx::SqlitePool;

use crate::core::Result;

const MAX_SCAN_CHARS: usize = 4096;
const MAX_PINYIN_CHARS: usize = 200;
const BACKFILL_BATCH_SIZE: i64 = 100;

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
