//! A migration runner for projects whose schema is hand-written `.sql` files.
//!
//! There is no migration tool in this family of services. Schema lives in plain
//! files that someone was supposed to run by hand, and that drifts: in gg_arcade
//! two files sat in the tree for months while the tables never existed in the
//! database, which is invisible until a `sqlx::query!` against one of them fails
//! to compile. This is the runner that closed that gap, moved here so the next
//! project does not have to write it again — prestige has nine such files, none
//! of them guarded, and no runner at all.
//!
//! The caller owns the ordered file list, because ordering is project knowledge:
//! every table carries REFERENCES to the ones above it, and a directory listing
//! gets that wrong the moment a filename changes. It also owns anything that has
//! to happen around the run — gg_arcade applies `bapesh::tasker`'s schema first,
//! because its `tasker_seed.sql` upserts into a table that module owns.
//!
//! ## What "applied" does and does not mean
//!
//! A statement counts as applied whenever it returns `Ok`, and
//! `CREATE TABLE IF NOT EXISTS` on an existing table returns `Ok`. So
//! `N applied, 0 already present` does **not** mean N things were created — a
//! file of fully-guarded statements reports the same numbers forever. Only
//! unguarded statements raise the duplicate-object SQLSTATEs and land in
//! *already present*.
//!
//! That tolerance is deliberate and is what lets a project adopt this without
//! first rewriting its SQL: unguarded legacy DDL runs, raises `42P07` on the
//! second pass, and is skipped. Guarding the files with `IF NOT EXISTS` is
//! better hygiene but is not a precondition.

use sqlx::Row;

use crate::db::{Pool, StdError};

/// Postgres SQLSTATEs meaning "this object is already there".
///
/// Only these are skipped. Anything else is a real failure and stops the run — a
/// migration that swallows arbitrary errors leaves a half-applied schema that
/// looks fine until the first query against the missing half.
fn is_already_exists(e: &sqlx::Error) -> bool {
    let Some(db) = e.as_database_error() else {
        return false;
    };
    let Some(code) = db.code() else { return false };
    matches!(
        code.as_ref(),
        "42P07" // duplicate_table
        | "42P06" // duplicate_schema
        | "42710" // duplicate_object (constraint, index, type)
        | "42701" // duplicate_column
    )
}

/// Splits a file into statements on top-level semicolons.
///
/// Tracks single quotes and `$tag$` dollar-quoting so a semicolon inside a
/// string literal or a function body does not split a statement in half. Line
/// (`--`) and block (`/* */`) comments are skipped for the same reason.
pub fn split_statements(sql: &str) -> Vec<String> {
    let b = sql.as_bytes();
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    // None = not inside anything; Some(tag) = inside a $tag$ ... $tag$ block.
    let mut dollar_tag: Option<String> = None;
    let mut in_single = false;
    // Whether anything since `start` is more than whitespace and comments.
    //
    // Without it, a file whose tail after the last semicolon is a comment block
    // emits that comment as a statement. Postgres answers an empty query rather
    // than erroring, so it never surfaced — it just quietly inflated the
    // "applied" count with a statement that did nothing, in a runner whose whole
    // job is telling you what actually got applied.
    let mut has_content = false;

    while i < b.len() {
        if let Some(tag) = &dollar_tag {
            if sql[i..].starts_with(tag.as_str()) {
                i += tag.len();
                dollar_tag = None;
            } else {
                i += 1;
            }
            continue;
        }
        if in_single {
            // '' is an escaped quote inside a literal, not the end of one.
            if b[i] == b'\'' {
                if sql[i + 1..].starts_with('\'') {
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }
        if sql[i..].starts_with("--") {
            i += sql[i..].find('\n').map(|n| n + 1).unwrap_or(sql.len() - i);
            continue;
        }
        if sql[i..].starts_with("/*") {
            i += sql[i..].find("*/").map(|n| n + 2).unwrap_or(sql.len() - i);
            continue;
        }
        if b[i] == b'\'' {
            in_single = true;
            has_content = true;
            i += 1;
            continue;
        }
        if b[i] == b'$' {
            // $$ or $tag$ — the tag is letters/digits/underscore.
            let rest = &sql[i + 1..];
            let end = rest.find('$');
            if let Some(end) = end {
                if rest[..end].chars().all(|c| c.is_alphanumeric() || c == '_') {
                    dollar_tag = Some(sql[i..i + end + 2].to_string());
                    has_content = true;
                    i += end + 2;
                    continue;
                }
            }
        }
        if b[i] == b';' {
            let stmt = sql[start..i].trim().to_string();
            // `has_content` and not merely a non-empty trim: the text between
            // two semicolons can be a comment, which trims to something and
            // executes to nothing.
            if has_content && !stmt.is_empty() {
                out.push(stmt);
            }
            start = i + 1;
            has_content = false;
            i += 1;
            continue;
        }
        if !b[i].is_ascii_whitespace() {
            has_content = true;
        }
        i += 1;
    }
    let tail = sql[start..].trim().to_string();
    if has_content && !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Applies `files`, in order, from `dir`.
///
/// A file that is not on disk is reported and skipped rather than failing the
/// run — which is friendly locally and **dangerous in a container**, because a
/// deploy that forgot to copy the schema directory reports `skip <file>: No such
/// file` for every one and otherwise looks like a clean no-op. That has
/// happened. If the run matters, check `applied + already_present` against the
/// file count, or call [`verify`] afterwards.
pub async fn run(pool: &Pool, dir: &str, files: &[&str]) -> Result<(), StdError> {
    println!("Applying {dir}/ ...");

    for file in files {
        let path = format!("{dir}/{file}");
        let sql = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                println!("  skip {file}: {e}");
                continue;
            }
        };

        let statements = split_statements(&sql);
        let (mut applied, mut skipped) = (0usize, 0usize);

        for stmt in &statements {
            match sqlx::query(stmt).execute(pool).await {
                Ok(_) => applied += 1,
                Err(e) if is_already_exists(&e) => skipped += 1,
                Err(e) => {
                    // The first line is enough to identify the statement without
                    // dumping a whole CREATE TABLE into the log.
                    let head = stmt.lines().next().unwrap_or("").trim();
                    return Err(format!("{file}: {head} ... failed: {e}").into());
                }
            }
        }
        println!("  {file}: {applied} applied, {skipped} already present");
    }
    Ok(())
}

/// Fails if any of `tables` is missing after a run.
///
/// The counterpart to `run`'s forgiving skip. `run` cannot tell "this file is
/// deliberately absent" from "the schema directory never made it into the
/// image", so it reports and continues; this turns the second case into the
/// non-zero exit a deploy can act on. Name the handful of tables the service
/// cannot serve a request without, not every table you have.
pub async fn verify(pool: &Pool, tables: &[&str]) -> Result<(), StdError> {
    let mut missing = Vec::new();
    for t in tables {
        let row = sqlx::query("SELECT to_regclass($1) IS NOT NULL AS present")
            .bind(t)
            .fetch_one(pool)
            .await?;
        if !row.try_get::<bool, _>("present").unwrap_or(false) {
            missing.push(*t);
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("schema is missing after migrate: {}", missing.join(", ")).into())
    }
}

#[cfg(test)]
mod tests {
    use super::split_statements;

    #[test]
    fn splits_plain_statements() {
        let s = split_statements("CREATE TABLE a (id INT);\nCREATE INDEX i ON a (id);\n");
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn ignores_semicolons_in_literals_and_comments() {
        let s = split_statements(
            "INSERT INTO a VALUES ('x;y');\n-- a comment; with one\n/* and; another */\nSELECT 1;",
        );
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("'x;y'"));
    }

    #[test]
    fn handles_escaped_quotes() {
        let s = split_statements("INSERT INTO a VALUES ('it''s; fine');\nSELECT 1;");
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn ignores_semicolons_in_dollar_quoted_bodies() {
        let s = split_statements(
            "DO $$ BEGIN PERFORM 1; PERFORM 2; END $$;\nCREATE TABLE a (id INT);",
        );
        assert_eq!(s.len(), 2);
        assert!(s[0].contains("PERFORM 2"));
    }

    /// A trailing statement with no semicolon is still a statement. Files in
    /// this family routinely end without one.
    #[test]
    fn a_trailing_statement_without_a_semicolon_still_counts() {
        let s = split_statements("SELECT 1;\nSELECT 2");
        assert_eq!(s.len(), 2);
        assert_eq!(s[1], "SELECT 2");
    }

    /// A file of nothing but comments produces no statements, rather than one
    /// empty one that would reach the database as a syntax error.
    #[test]
    fn comments_alone_produce_nothing() {
        assert!(split_statements("-- just a note\n/* and another */\n").is_empty());
    }
}
