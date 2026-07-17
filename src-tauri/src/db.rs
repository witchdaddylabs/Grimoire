use crate::errors::CommandResult;
use crate::models::{
    BannedWord, SearchChunkResult, VaultDrawerNode, VaultHallNode, VaultItemDetail, VaultItemNode,
    VaultRoomNode, VaultTreeResponse, VaultWingNode, WardScanResponse,
};
use rusqlite::{params, Connection, Params};


// -- Re-exports used by vault.rs / wards.rs --

pub fn collect_named_rows<P>(
    connection: &Connection,
    query: &str,
    params: P,
) -> CommandResult<Vec<(String, String, Option<String>)>>
where
    P: Params,
{
    let mut statement = connection
        .prepare(query)
        .map_err(|error| format!("Could not prepare vault tree query: {error}"))?;
    let mapped = statement
        .query_map(params, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(|error| format!("Could not query vault tree: {error}"))?;

    let mut rows = Vec::new();
    for row in mapped {
        rows.push(row.map_err(|error| format!("Could not read vault tree row: {error}"))?);
    }

    Ok(rows)
}

pub fn count_words(text: &str) -> i64 {
    text.split_whitespace().count() as i64
}

pub fn read_vault_tree(connection: &Connection) -> CommandResult<VaultTreeResponse> {
    let wing_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM wings ORDER BY sort_order, name",
        [],
    )?;

    let mut item_count = 0;
    let mut wings = Vec::new();

    for (wing_id, wing_name, wing_description) in wing_rows {
        let halls = read_halls(connection, &wing_id, &wing_name, &mut item_count)?;
        wings.push(VaultWingNode {
            id: wing_id,
            name: wing_name,
            description: wing_description,
            halls,
        });
    }

    Ok(VaultTreeResponse { wings, item_count })
}

fn read_halls(
    connection: &Connection,
    wing_id: &str,
    wing_name: &str,
    item_count: &mut usize,
) -> CommandResult<Vec<VaultHallNode>> {
    let hall_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM halls WHERE wing_id = ?1 ORDER BY sort_order, name",
        params![wing_id],
    )?;

    let mut halls = Vec::new();
    for (hall_id, hall_name, hall_description) in hall_rows {
        let rooms = read_rooms(connection, &hall_id, wing_name, &hall_name, item_count)?;
        halls.push(VaultHallNode {
            id: hall_id,
            name: hall_name,
            description: hall_description,
            rooms,
        });
    }

    Ok(halls)
}

fn read_rooms(
    connection: &Connection,
    hall_id: &str,
    wing_name: &str,
    hall_name: &str,
    item_count: &mut usize,
) -> CommandResult<Vec<VaultRoomNode>> {
    let room_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM rooms WHERE hall_id = ?1 ORDER BY sort_order, name",
        params![hall_id],
    )?;

    let mut rooms = Vec::new();
    for (room_id, room_name, room_description) in room_rows {
        let drawers = read_drawers(
            connection,
            &room_id,
            wing_name,
            hall_name,
            &room_name,
            item_count,
        )?;
        rooms.push(VaultRoomNode {
            id: room_id,
            name: room_name,
            description: room_description,
            drawers,
        });
    }

    Ok(rooms)
}

fn read_drawers(
    connection: &Connection,
    room_id: &str,
    wing_name: &str,
    hall_name: &str,
    room_name: &str,
    item_count: &mut usize,
) -> CommandResult<Vec<VaultDrawerNode>> {
    let drawer_rows = collect_named_rows(
        connection,
        "SELECT id, name, description FROM drawers WHERE room_id = ?1 ORDER BY sort_order, name",
        params![room_id],
    )?;

    let mut drawers = Vec::new();
    for (drawer_id, drawer_name, drawer_description) in drawer_rows {
        let items = read_items(
            connection,
            &drawer_id,
            wing_name,
            hall_name,
            room_name,
            &drawer_name,
        )?;
        *item_count += items.len();
        drawers.push(VaultDrawerNode {
            id: drawer_id,
            name: drawer_name,
            description: drawer_description,
            items,
        });
    }

    Ok(drawers)
}

fn read_items(
    connection: &Connection,
    drawer_id: &str,
    wing_name: &str,
    hall_name: &str,
    room_name: &str,
    drawer_name: &str,
) -> CommandResult<Vec<VaultItemNode>> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, title, item_type, content, word_count
            FROM items
            WHERE drawer_id = ?1
              AND archived_at IS NULL
            ORDER BY sort_order, title
            "#,
        )
        .map_err(|error| format!("Could not prepare item query: {error}"))?;

    let mapped = statement
        .query_map(params![drawer_id], |row| {
            let title: String = row.get(1)?;
            Ok(VaultItemNode {
                id: row.get(0)?,
                path: format!("{wing_name} / {hall_name} / {room_name} / {drawer_name} / {title}"),
                title,
                item_type: row.get(2)?,
                content: row.get(3)?,
                word_count: row.get(4)?,
            })
        })
        .map_err(|error| format!("Could not query vault items: {error}"))?;

    let mut items = Vec::new();
    for item in mapped {
        items.push(item.map_err(|error| format!("Could not read vault item: {error}"))?);
    }

    Ok(items)
}

pub fn read_item_detail(
    connection: &Connection,
    item_id: &str,
) -> CommandResult<VaultItemDetail> {
    connection
        .query_row(
            r#"
            SELECT
              i.id,
              i.title,
              i.item_type,
              COALESCE(i.content, ''),
              COALESCE(i.plain_text, ''),
              i.word_count,
              i.updated_at,
              w.name,
              h.name,
              r.name,
              d.name
            FROM items i
            JOIN drawers d ON d.id = i.drawer_id
            JOIN rooms r ON r.id = d.room_id
            JOIN halls h ON h.id = r.hall_id
            JOIN wings w ON w.id = h.wing_id
            WHERE i.id = ?1
              AND i.archived_at IS NULL
            "#,
            params![item_id],
            |row| {
                let title: String = row.get(1)?;
                let wing: String = row.get(7)?;
                let hall: String = row.get(8)?;
                let room: String = row.get(9)?;
                let drawer: String = row.get(10)?;
                Ok(VaultItemDetail {
                    id: row.get(0)?,
                    title: title.clone(),
                    item_type: row.get(2)?,
                    content: row.get(3)?,
                    plain_text: row.get(4)?,
                    word_count: row.get(5)?,
                    updated_at: row.get(6)?,
                    path: format!("{wing} / {hall} / {room} / {drawer} / {title}"),
                })
            },
        )
        .map_err(|error| format!("Could not read Canvas item: {error}"))
}

pub fn item_path(connection: &Connection, item_id: &str, title: &str) -> CommandResult<String> {
    connection
        .query_row(
            r#"
            SELECT w.name, h.name, r.name, d.name
            FROM items i
            JOIN drawers d ON d.id = i.drawer_id
            JOIN rooms r ON r.id = d.room_id
            JOIN halls h ON h.id = r.hall_id
            JOIN wings w ON w.id = h.wing_id
            WHERE i.id = ?1
              AND i.archived_at IS NULL
            "#,
            params![item_id],
            |row| {
                let wing: String = row.get(0)?;
                let hall: String = row.get(1)?;
                let room: String = row.get(2)?;
                let drawer: String = row.get(3)?;
                Ok(format!("{wing} / {hall} / {room} / {drawer} / {title}"))
            },
        )
        .map_err(|error| format!("Could not resolve vault path: {error}"))
}

pub fn item_type(connection: &Connection, item_id: &str) -> CommandResult<String> {
    connection
        .query_row(
            "SELECT item_type FROM items WHERE id = ?1 AND archived_at IS NULL",
            params![item_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not resolve item type: {error}"))
}

pub fn ensure_import_drawer(connection: &Connection) -> CommandResult<String> {
    let now = super::timestamp();
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO wings (id, name, description, sort_order, created_at, updated_at)
            VALUES ('wing_imports', 'The Vault', 'Imported writing and notes', 99, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import wing: {error}"))?;
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO halls (id, wing_id, name, description, sort_order, created_at, updated_at)
            VALUES ('hall_feed', 'wing_imports', 'Feed', 'Material brought into the vault', 0, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import hall: {error}"))?;
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO rooms (id, hall_id, name, description, sort_order, created_at, updated_at)
            VALUES ('room_imports', 'hall_feed', 'Imports', 'Text and Markdown imports', 0, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import room: {error}"))?;
    connection
        .execute(
            r#"
            INSERT OR IGNORE INTO drawers (id, room_id, name, description, sort_order, created_at, updated_at)
            VALUES ('drawer_imported_text', 'room_imports', 'Imported Text', 'Newly imported writing', 0, ?1, ?1)
            "#,
            params![now],
        )
        .map_err(|error| format!("Could not prepare import drawer: {error}"))?;
    Ok("drawer_imported_text".to_string())
}

pub fn next_sort_order(
    connection: &Connection,
    table: &str,
    parent_column: &str,
    parent_id: &str,
) -> CommandResult<i64> {
    let query = format!("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM {table} WHERE {parent_column} = ?1");
    connection
        .query_row(&query, params![parent_id], |row| row.get(0))
        .map_err(|error| format!("Could not calculate sort order: {error}"))
}

pub fn clear_item_chunks(connection: &Connection, item_id: &str) -> CommandResult<()> {
    connection
        .execute(
            "DELETE FROM item_chunks_fts WHERE item_id = ?1",
            params![item_id],
        )
        .map_err(|error| format!("Could not clear search index: {error}"))?;
    connection
        .execute(
            "DELETE FROM item_chunks WHERE item_id = ?1",
            params![item_id],
        )
        .map_err(|error| format!("Could not clear item chunks: {error}"))?;
    Ok(())
}

pub fn sync_item_chunks(
    connection: &Connection,
    item_id: &str,
    title: &str,
    item_type: &str,
    vault_path: &str,
    text: &str,
) -> CommandResult<usize> {
    clear_item_chunks(connection, item_id)?;
    let chunks = super::chunk_text(text, 240);
    let now = super::timestamp();
    for (index, chunk) in chunks.iter().enumerate() {
        let chunk_id = format!("{item_id}_chunk_{index}");
        connection
            .execute(
                r#"
                INSERT INTO item_chunks (
                  id, item_id, chunk_index, text, word_count, start_offset,
                  end_offset, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?6)
                "#,
                params![
                    chunk_id,
                    item_id,
                    index as i64,
                    chunk,
                    count_words(chunk),
                    now
                ],
            )
            .map_err(|error| format!("Could not write item chunk: {error}"))?;
        connection
            .execute(
                r#"
                INSERT INTO item_chunks_fts (
                  chunk_id, item_id, title, item_type, vault_path, text
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
                params![chunk_id, item_id, title, item_type, vault_path, chunk],
            )
            .map_err(|error| format!("Could not update search index: {error}"))?;
    }

    Ok(chunks.len())
}

pub fn normalize_text(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut blank_lines = 0;
    let mut lines = Vec::new();
    for line in normalized.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.trim().is_empty() {
            blank_lines += 1;
            if blank_lines <= 2 {
                lines.push(String::new());
            }
        } else {
            blank_lines = 0;
            lines.push(trimmed_end.to_string());
        }
    }

    lines.join("\n").trim().to_string()
}

pub fn read_banned_words(connection: &Connection) -> CommandResult<Vec<BannedWord>> {
    let mut statement = connection
        .prepare("SELECT id, value, severity, is_default FROM banned_words ORDER BY is_default DESC, value")
        .map_err(|error| format!("Could not prepare ward list: {error}"))?;
    let mapped = statement
        .query_map([], |row| {
            let is_default: i64 = row.get(3)?;
            Ok(BannedWord {
                id: row.get(0)?,
                value: row.get(1)?,
                severity: row.get(2)?,
                is_default: is_default == 1,
            })
        })
        .map_err(|error| format!("Could not query ward list: {error}"))?;
    let mut words = Vec::new();
    for word in mapped {
        words.push(word.map_err(|error| format!("Could not read ward phrase: {error}"))?);
    }
    Ok(words)
}

pub fn add_banned_word(
    connection: &Connection,
    value: &str,
    severity: &str,
) -> CommandResult<Vec<BannedWord>> {
    let now = super::timestamp();
    connection
        .execute(
            r#"
            INSERT INTO banned_words (id, value, severity, is_default, created_at, updated_at)
            VALUES (?1, ?2, ?3, 0, ?4, ?4)
            ON CONFLICT(value) DO UPDATE SET severity = excluded.severity, updated_at = excluded.updated_at
            "#,
            params![format!("ward_{}", super::timestamp_nanos()), value, severity, now],
        )
        .map_err(|error| format!("Could not save ward phrase: {error}"))?;

    read_banned_words(connection)
}

pub fn remove_banned_word(connection: &Connection, id: &str) -> CommandResult<Vec<BannedWord>> {
    connection
        .execute("DELETE FROM banned_words WHERE id = ?1", params![id])
        .map_err(|error| format!("Could not remove ward phrase: {error}"))?;
    read_banned_words(connection)
}

// -- Re-export of llms helpers used by tests / wards --
pub fn scan_banned_words(words: &[BannedWord], text: &str) -> crate::models::WardScanResponse {
    super::llm::scan_wards(words, text)
}

// -- Internal helpers for chat_with_vault orchestration --

pub fn fts_query_terms(query: &str) -> CommandResult<String> {
    let mut terms: Vec<String> = Vec::new();
    for raw in query.split_whitespace() {
        let trimmed = raw.trim_matches(|c: char| !c.is_alphanumeric());
        if trimmed.is_empty() {
            continue;
        }
        let mut escaped = String::new();
        for ch in trimmed.chars() {
            if ch == '"' || ch == '*' || ch == '(' || ch == ')' {
                escaped.push('"');
                escaped.push(ch);
                escaped.push('"');
            } else {
                escaped.push(ch);
            }
        }
        terms.push(format!("{}*", escaped));
    }
    if terms.is_empty() {
        return Err("Query is empty.".to_string());
    }
    Ok(terms.join(" "))
}

pub fn search_chunks_internal(
    connection: &Connection,
    query: &str,
    limit: i64,
) -> CommandResult<Vec<SearchChunkResult>> {
    let fts_query = fts_query_terms(query)?;
    let limit = limit.clamp(1, 24);
    let mut statement = connection
        .prepare(
            r#"
            SELECT
              chunk_id,
              item_id,
              title,
              item_type,
              vault_path,
              text,
              bm25(item_chunks_fts) AS rank
            FROM item_chunks_fts
            WHERE item_chunks_fts MATCH ?1
            ORDER BY rank
            LIMIT ?2
            "#,
        )
        .map_err(|error| format!("Could not prepare Vault search: {error}"))?;

    let mapped = statement
        .query_map(params![fts_query, limit], |row| {
            let raw_score: f64 = row.get(6)?;
            let score = 0.0 - raw_score;
            Ok(SearchChunkResult {
                chunk_id: row.get(0)?,
                item_id: row.get(1)?,
                title: row.get(2)?,
                item_type: row.get(3)?,
                vault_path: row.get(4)?,
                snippet: row.get(5)?,
                score,
                confidence: crate::llm::confidence_for_score(score),
            })
        })
        .map_err(|error| format!("Could not search Vault chunks: {error}"))?;

    let mut results = Vec::new();
    for result in mapped {
        results.push(result.map_err(|error| format!("Could not read search result: {error}"))?);
    }
    Ok(results)
}

pub fn scan_wards_internal(
    connection: &Connection,
    text: &str,
) -> CommandResult<WardScanResponse> {
    let words = read_banned_words(connection)?;
    Ok(crate::llm::scan_wards(&words, text))
}
