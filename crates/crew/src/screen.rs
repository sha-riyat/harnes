//! Reads an agent's terminal screen like a computer-use agent reads an
//! accessibility tree: finds the question or menu waiting for the user so
//! the UI can show it (and answer it) without displaying the terminal.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Choice {
    /// Keys to send to select this choice.
    pub keys: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScreenPrompt {
    pub question: String,
    pub choices: Vec<Choice>,
}

const FRAME: &[char] = &['│', '┃', '║', '╭', '╮', '╰', '╯', '─', '━', '═', '|', ' '];

fn clean(line: &str) -> &str {
    line.trim_matches(FRAME).trim()
}

const POINTERS: [char; 5] = ['❯', '>', '›', '●', '○'];

/// The option under the menu's cursor (`❯ 2. …`).
fn pointed(line: &str) -> bool {
    clean(line).starts_with(['❯', '>', '›'])
}

/// Keys that move a menu's cursor from `from` to `to`, then confirm.
/// Menus (Claude Code's included) follow the arrows; digits alone are not
/// enough everywhere, so the answer always ends with Enter.
fn choose(from: usize, to: usize) -> String {
    let step = if to > from { "\u{1b}[B" } else { "\u{1b}[A" };
    format!("{}\r", step.repeat(from.abs_diff(to)))
}

/// `❯ 1. Yes` / `  2. No` -> (number, label)
fn option(line: &str) -> Option<(u32, String)> {
    let l = clean(line).trim_start_matches(POINTERS).trim_start();
    let (num, rest) = l.split_once('.')?;
    let n: u32 = num.trim().parse().ok()?;
    let label = rest.trim();
    (n >= 1 && n <= 9 && !label.is_empty()).then(|| (n, label.to_string()))
}

/// Finds a numbered menu (2+ consecutive options) or a `(y/n)` prompt in the
/// visible screen, scanning from the bottom (the most recent output).
pub fn detect(screen: &str) -> Option<ScreenPrompt> {
    let lines: Vec<&str> = screen.lines().collect();
    let mut i = lines.len();
    while i > 0 {
        i -= 1;
        if option(lines[i]).is_none() {
            let l = clean(lines[i]).to_lowercase();
            if l.ends_with("(y/n)") || l.ends_with("[y/n]") || l.ends_with("[y/n]?") || l.ends_with("(y/n)?") {
                return Some(ScreenPrompt {
                    question: clean(lines[i]).to_string(),
                    choices: vec![Choice { keys: "y\r".into(), label: "Oui".into() }, Choice { keys: "n\r".into(), label: "Non".into() }],
                });
            }
            continue;
        }
        // Walk up through the whole option block.
        let end = i;
        let mut start = i;
        while start > 0 && option(lines[start - 1]).is_some() {
            start -= 1;
        }
        let opts: Vec<(u32, String)> = (start..=end).filter_map(|k| option(lines[k])).collect();
        let sequential = opts.windows(2).all(|w| w[1].0 == w[0].0 + 1) && opts.first().map(|o| o.0) == Some(1);
        if opts.len() >= 2 && sequential {
            let question = (0..start)
                .rev()
                .map(|k| clean(lines[k]))
                .find(|l| !l.is_empty())
                .unwrap_or("Question de l'agent")
                .to_string();
            let at = (start..=end).position(|k| pointed(lines[k])).unwrap_or(0);
            return Some(ScreenPrompt {
                question,
                choices: opts.into_iter().enumerate().map(|(j, (_, label))| Choice { keys: choose(at, j), label }).collect(),
            });
        }
        i = start;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_claude_permission_menu() {
        let screen = "\
╭──────────────────────────────╮
│ Bash command                 │
│   npm test                   │
│ Do you want to proceed?      │
│ ❯ 1. Yes                     │
│   2. Yes, and don't ask again│
│   3. No, and tell Claude what│
╰──────────────────────────────╯";
        let p = detect(screen).expect("menu");
        assert_eq!(p.question, "Do you want to proceed?");
        assert_eq!(p.choices.len(), 3);
        assert_eq!(p.choices[1], Choice { keys: "\u{1b}[B\r".into(), label: "Yes, and don't ask again".into() });
        assert_eq!(p.choices[0].keys, "\r", "the pointed option only needs Enter");
    }

    #[test]
    fn moves_from_wherever_the_cursor_is() {
        let p = detect("Pick one\n  1. a\n  2. b\n❯ 3. c\n  4. d").expect("menu");
        assert_eq!(p.choices[0].keys, "\u{1b}[A\u{1b}[A\r");
        assert_eq!(p.choices[3].keys, "\u{1b}[B\r");
    }

    #[test]
    fn reads_yes_no_prompt_and_ignores_plain_lists() {
        assert_eq!(detect("Overwrite file? (y/N)").unwrap().choices[0].keys, "y\r");
        assert!(detect("Steps done:\n1. lint\nnothing else").is_none(), "a single item is not a menu");
        assert!(detect("2. second\n3. third").is_none(), "menus start at 1");
    }
}
