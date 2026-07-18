use super::*;
use rusqlite::params;

#[tauri::command]
pub fn db_search_chunks(request: SearchChunksRequest) -> CommandResult<SearchChunksResponse> {
    let connection = open_project_database(&request.project_path)?;
    let query = request.query.trim().to_string();
    let fts = if request.mode.as_deref() == Some("broad") {
        fts_query_broad(&query)?
    } else {
        fts_query(&query)?
    };
    let limit = request.limit.unwrap_or(8).clamp(1, 24);
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
        .query_map(params![fts, limit], |row| {
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
                confidence: confidence_for_score(score),
            })
        })
        .map_err(|error| format!("Could not search Vault chunks: {error}"))?;

    let mut results = Vec::new();
    for result in mapped {
        results.push(result.map_err(|error| format!("Could not read search result: {error}"))?);
    }

    Ok(SearchChunksResponse {
        query,
        confidence: aggregate_confidence(&results),
        results,
    })
}

fn fts_query(query: &str) -> CommandResult<String> {
    let tokens = search_tokens(query);
    if tokens.is_empty() {
        return Err("Search needs at least one word or number.".to_string());
    }

    Ok(tokens
        .into_iter()
        .take(8)
        .map(|token| format!("{token}*"))
        .collect::<Vec<_>>()
        .join(" "))
}

fn fts_query_broad(query: &str) -> CommandResult<String> {
    let filtered = search_tokens(query)
        .into_iter()
        .filter(|token| !vault_recall_stopword(token))
        .collect::<Vec<_>>();
    let tokens = if filtered.is_empty() {
        search_tokens(query)
    } else {
        filtered
    };

    if tokens.is_empty() {
        return Err("Search needs at least one word or number.".to_string());
    }

    Ok(tokens
        .into_iter()
        .take(12)
        .map(|token| format!("{token}*"))
        .collect::<Vec<_>>()
        .join(" OR "))
}

fn search_tokens(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in query.chars() {
        if character.is_alphanumeric() || character == '_' {
            current.push(character);
        } else if !current.is_empty() {
            tokens.push(current.to_lowercase());
            current.clear();
        }
    }
    if !current.is_empty() {
        tokens.push(current.to_lowercase());
    }

    tokens
}

fn vault_recall_stopword(token: &str) -> bool {
    matches!(
        token,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "about"
            | "be"
            | "because"
            | "but"
            | "by"
            | "can"
            | "could"
            | "do"
            | "does"
            | "for"
            | "from"
            | "have"
            | "how"
            | "i"
            | "in"
            | "is"
            | "it"
            | "its"
            | "me"
            | "of"
            | "on"
            | "or"
            | "please"
            | "should"
            | "tell"
            | "that"
            | "the"
            | "their"
            | "there"
            | "this"
            | "to"
            | "was"
            | "what"
            | "when"
            | "where"
            | "which"
            | "who"
            | "why"
            | "with"
            | "would"
            | "you"
    )
}

fn confidence_for_score(score: f64) -> String {
    if score >= 8.0 {
        "high".to_string()
    } else if score >= 3.0 {
        "medium".to_string()
    } else if score > 0.0 {
        "low".to_string()
    } else {
        "none".to_string()
    }
}

fn aggregate_confidence(results: &[SearchChunkResult]) -> String {
    results
        .first()
        .map(|result| result.confidence.clone())
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_query_sanitizes_and_suffixes_terms() {
        assert_eq!(fts_query("Mara's bell!").unwrap(), "mara* s* bell*");
    }

    #[test]
    fn broad_fts_query_uses_recall_terms() {
        assert_eq!(
            fts_query_broad("What is the secret name from the other file?").unwrap(),
            "secret* OR name* OR other* OR file*"
        );
    }
}
