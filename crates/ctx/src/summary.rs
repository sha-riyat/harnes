//! Turns a large output into a compact, model-friendly digest.

use crate::{store::Store, INLINE_LIMIT};
use anyhow::Result;

const HEAD: usize = 15;
const TAIL: usize = 10;

/// Lines that usually matter in build/test output.
fn is_signal(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    [
        "error",
        "fail",
        "panic",
        "exception",
        "warning",
        "traceback",
        "assert",
        "fatal",
    ]
    .iter()
    .any(|k| l.contains(k))
}

/// Inline small outputs; store, index and summarise large ones.
pub fn digest(store: &Store, source: &str, content: &str, intent: Option<&str>) -> Result<String> {
    if content.len() <= INLINE_LIMIT {
        store.add_returned(content.len());
        return Ok(content.to_string());
    }
    let id = store.put(source, content)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut out = format!(
        "[harnes-ctx] {source}: {} lines, {} bytes stored as id={id} (kept out of context).\n",
        lines.len(),
        content.len()
    );

    out.push_str("\n--- head ---\n");
    out.push_str(
        &lines
            .iter()
            .take(HEAD)
            .copied()
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let signals: Vec<String> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_signal(l))
        .take(20)
        .map(|(i, l)| format!("{:>6}: {}", i + 1, truncate(l, 200)))
        .collect();
    if !signals.is_empty() {
        out.push_str("\n--- errors / warnings ---\n");
        out.push_str(&signals.join("\n"));
    }

    if lines.len() > HEAD {
        out.push_str("\n--- tail ---\n");
        let start = lines.len().saturating_sub(TAIL).max(HEAD);
        out.push_str(&lines[start..].join("\n"));
    }

    if let Some(q) = intent.filter(|q| !q.trim().is_empty()) {
        let hits: Vec<_> = store
            .search(q, 8)?
            .into_iter()
            .filter(|h| h.id == id)
            .take(5)
            .collect();
        if !hits.is_empty() {
            out.push_str(&format!("\n--- matches for \"{q}\" ---\n"));
            for h in hits {
                out.push_str(&format!(
                    "line {}: {}\n",
                    h.line,
                    h.snippet.replace('\n', " ")
                ));
            }
        }
    }
    out.push_str(&format!(
        "\n\nUse ctx_search (query) or ctx_retrieve (id={id}, offset, limit) to read more."
    ));
    store.add_returned(out.len());
    Ok(out)
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}
