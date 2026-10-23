//! Typed questions from an agent to the user: the `ask_user` MCP tool.
//!
//! Until now an agent that wanted a decision printed a menu in its terminal and
//! harnes read the screen to find it. That works, but it is fragile: a menu
//! with an unusual layout is never seen. Here the agent states the question as
//! JSON in one of three registered kinds, the cockpit draws it, and the answer
//! goes straight back. A kind that is not registered is refused, never
//! guessed: the model describes what it needs, it never describes the UI.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::oneshot;

pub const MAX_QUESTION: usize = 500;
pub const MAX_LABEL: usize = 80;
pub const MAX_DETAIL: usize = 200;
pub const MAX_OPTIONS: usize = 8;
pub const MAX_TEXT: usize = 2_000;
/// Questions waiting at once, over every agent.
pub const MAX_PENDING: usize = 20;
pub const DEFAULT_WAIT: u64 = 300;
/// Kept below the tool timeout of MCP clients.
pub const MAX_WAIT: u64 = 540;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// One of a few options.
    Choice,
    /// Yes or no.
    Confirm,
    /// A free answer.
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub detail: String,
}

/// What the agent sends. `deny_unknown_fields`: a field the registry does not
/// know is an error the agent sees, not something silently dropped.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub kind: Kind,
    pub question: String,
    #[serde(default)]
    pub options: Vec<Choice>,
    /// The id (choice), `yes` or `no` (confirm), or the text (text) that stands
    /// if the user does not answer in time.
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

fn short(s: &str, max: usize, what: &str) -> Result<(), String> {
    let n = s.chars().count();
    if s.trim().is_empty() {
        return Err(format!("{what} est vide"));
    }
    if n > max {
        return Err(format!("{what} dépasse {max} caractères ({n})"));
    }
    Ok(())
}

impl Card {
    /// Parses and validates what an agent sent. The error is written for the agent.
    pub fn parse(v: &Value) -> Result<Card, String> {
        let kind = v.get("kind").and_then(Value::as_str).unwrap_or("");
        if !["choice", "confirm", "text"].contains(&kind) {
            return Err(format!("kind `{kind}` inconnu : choice, confirm ou text"));
        }
        let card: Card = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
        short(&card.question, MAX_QUESTION, "question")?;
        match card.kind {
            Kind::Choice => {
                if !(2..=MAX_OPTIONS).contains(&card.options.len()) {
                    return Err(format!(
                        "un choix a de 2 à {MAX_OPTIONS} options ({} reçues)",
                        card.options.len()
                    ));
                }
                let mut seen = std::collections::HashSet::new();
                for o in &card.options {
                    short(&o.id, 40, "l'id d'une option")?;
                    short(&o.label, MAX_LABEL, "le libellé d'une option")?;
                    if o.detail.chars().count() > MAX_DETAIL {
                        return Err(format!(
                            "le détail d'une option dépasse {MAX_DETAIL} caractères"
                        ));
                    }
                    if !seen.insert(o.id.as_str()) {
                        return Err(format!("id d'option en double : {}", o.id));
                    }
                }
            }
            Kind::Confirm | Kind::Text if !card.options.is_empty() => {
                return Err("`options` n'existe que pour un choix".into());
            }
            _ => {}
        }
        if let Some(d) = &card.default {
            let ok = match card.kind {
                Kind::Choice => card.options.iter().any(|o| &o.id == d),
                Kind::Confirm => d == "yes" || d == "no",
                Kind::Text => d.chars().count() <= MAX_TEXT,
            };
            if !ok {
                return Err("`default` ne correspond à aucune réponse possible".into());
            }
        }
        Ok(card)
    }

    pub fn wait(&self) -> Duration {
        Duration::from_secs(self.timeout_secs.unwrap_or(DEFAULT_WAIT).clamp(5, MAX_WAIT))
    }
}

/// A question waiting for the user.
#[derive(Debug, Clone, Serialize)]
pub struct Pending {
    pub id: String,
    pub agent: String,
    pub card: Card,
    pub at: u64,
}

/// What the user answered. For a confirm, `choice` is `yes` or `no`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    #[serde(default)]
    pub choice: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

/// What the agent gets back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Answered(Answer),
    /// Nobody answered in time; the card's `default`, if any, applies.
    TimedOut(Option<String>),
    /// The user chose not to answer.
    Declined,
    /// The agent's pane went away.
    Cancelled,
}

impl Outcome {
    pub fn json(&self) -> Value {
        match self {
            Outcome::Answered(a) => json!({"answered": true, "choice": a.choice, "text": a.text}),
            Outcome::TimedOut(d) => json!({"answered": false, "reason": "timeout", "default": d}),
            Outcome::Declined => json!({"answered": false, "reason": "declined"}),
            Outcome::Cancelled => json!({"answered": false, "reason": "cancelled"}),
        }
    }
}

pub type Sink = Arc<dyn Fn(&Pending) + Send + Sync>;

#[derive(Default)]
pub struct Asks {
    pending: Mutex<HashMap<String, (Pending, oneshot::Sender<Outcome>)>>,
    sink: std::sync::OnceLock<Sink>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Asks {
    pub fn set_sink(&self, f: Sink) {
        let _ = self.sink.set(f);
    }

    pub fn pending(&self) -> Vec<Pending> {
        let mut v: Vec<Pending> = self
            .pending
            .lock()
            .values()
            .map(|(p, _)| p.clone())
            .collect();
        v.sort_by_key(|p| p.at);
        v
    }

    /// Delivers the user's answer after checking it against the card.
    pub fn answer(&self, id: &str, a: Answer) -> Result<(), String> {
        let mut map = self.pending.lock();
        let Some((p, _)) = map.get(id) else {
            return Err("question expirée".into());
        };
        let checked = match p.card.kind {
            Kind::Choice => {
                let c = a.choice.clone().ok_or("choisis une option")?;
                if !p.card.options.iter().any(|o| o.id == c) {
                    return Err("option inconnue".into());
                }
                Answer {
                    choice: Some(c),
                    text: None,
                }
            }
            Kind::Confirm => match a.choice.as_deref() {
                Some(c @ ("yes" | "no")) => Answer {
                    choice: Some(c.into()),
                    text: None,
                },
                _ => return Err("réponds yes ou no".into()),
            },
            Kind::Text => {
                let t = a.text.clone().unwrap_or_default();
                if t.trim().is_empty() {
                    return Err("la réponse est vide".into());
                }
                if t.chars().count() > MAX_TEXT {
                    return Err(format!("la réponse dépasse {MAX_TEXT} caractères"));
                }
                Answer {
                    choice: None,
                    text: Some(t),
                }
            }
        };
        let (_, tx) = map.remove(id).expect("checked above");
        tx.send(Outcome::Answered(checked))
            .map_err(|_| "question expirée".to_string())
    }

    /// The user chose not to answer.
    pub fn decline(&self, id: &str) -> bool {
        self.pending
            .lock()
            .remove(id)
            .is_some_and(|(_, tx)| tx.send(Outcome::Declined).is_ok())
    }

    /// Drops every question of an agent whose pane went away.
    pub fn cancel_agent(&self, agent: &str) {
        let mut map = self.pending.lock();
        let ids: Vec<String> = map
            .iter()
            .filter(|(_, (p, _))| p.agent == agent)
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some((_, tx)) = map.remove(&id) {
                let _ = tx.send(Outcome::Cancelled);
            }
        }
    }

    /// Registers a question and waits for the user. Errors are for the agent:
    /// a bad card, too many questions at once.
    pub async fn request(&self, agent: &str, raw: &Value) -> Result<Outcome, String> {
        self.request_for(agent, raw, None).await
    }

    /// `request` with the wait fixed by the caller (tests).
    pub async fn request_for(
        &self,
        agent: &str,
        raw: &Value,
        wait: Option<Duration>,
    ) -> Result<Outcome, String> {
        let card = Card::parse(raw)?;
        let wait = wait.unwrap_or_else(|| card.wait());
        let default = card.default.clone();
        let id = format!(
            "a{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let pending = Pending {
            id: id.clone(),
            agent: agent.into(),
            card,
            at: now_ms(),
        };
        let rx = {
            let mut map = self.pending.lock();
            if map.len() >= MAX_PENDING {
                return Err(
                    "trop de questions en attente : réessaie plus tard ou décide seul".into(),
                );
            }
            if map.values().any(|(p, _)| p.agent == agent) {
                return Err("tu as déjà une question en attente : attends sa réponse".into());
            }
            let (tx, rx) = oneshot::channel();
            map.insert(id.clone(), (pending.clone(), tx));
            rx
        };
        if let Some(sink) = self.sink.get() {
            sink(&pending);
        }
        let out = tokio::time::timeout(wait, rx).await;
        self.pending.lock().remove(&id);
        Ok(match out {
            Ok(Ok(o)) => o,
            Ok(Err(_)) => Outcome::Cancelled,
            Err(_) => Outcome::TimedOut(default),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_registered_kinds_and_well_formed_cards_are_accepted() {
        let ok = json!({"kind": "choice", "question": "Which?", "options": [{"id": "a", "label": "A"}, {"id": "b", "label": "B", "detail": "slower"}], "default": "a"});
        assert!(Card::parse(&ok).is_ok());
        assert!(Card::parse(&json!({"kind": "slider", "question": "x"}))
            .unwrap_err()
            .contains("inconnu"));
        assert!(
            Card::parse(&json!({"kind": "confirm", "question": "x", "html": "<b>"})).is_err(),
            "unknown field"
        );
        assert!(
            Card::parse(
                &json!({"kind": "choice", "question": "x", "options": [{"id": "a", "label": "A"}]})
            )
            .is_err(),
            "one option"
        );
        assert!(Card::parse(&json!({"kind": "choice", "question": "x", "options": [{"id": "a", "label": "A"}, {"id": "a", "label": "B"}]})).unwrap_err().contains("double"));
        assert!(Card::parse(
            &json!({"kind": "confirm", "question": "x", "options": [{"id": "a", "label": "A"}]})
        )
        .is_err());
        assert!(
            Card::parse(&json!({"kind": "confirm", "question": "x", "default": "maybe"})).is_err()
        );
        assert!(
            Card::parse(&json!({"kind": "text", "question": "q".repeat(MAX_QUESTION + 1)}))
                .is_err()
        );
        assert!(Card::parse(&json!({"kind": "text", "question": " "})).is_err());
    }

    async fn ask(
        asks: &Arc<Asks>,
        agent: &str,
        card: Value,
    ) -> tokio::task::JoinHandle<Result<Outcome, String>> {
        let (a, agent, seen) = (asks.clone(), agent.to_string(), asks.pending().len());
        let h = tokio::spawn(async move { a.request(&agent, &card).await });
        while asks.pending().len() == seen {
            tokio::task::yield_now().await;
        }
        h
    }

    #[tokio::test]
    async fn a_choice_is_answered_and_checked() {
        let asks = Arc::new(Asks::default());
        let h = ask(&asks, "w1", json!({"kind": "choice", "question": "Which?", "options": [{"id": "a", "label": "A"}, {"id": "b", "label": "B"}]})).await;
        let id = asks.pending()[0].id.clone();
        assert!(
            asks.answer(
                &id,
                Answer {
                    choice: Some("zzz".into()),
                    text: None
                }
            )
            .is_err(),
            "not an option"
        );
        assert!(
            asks.answer(&id, Answer::default()).is_err(),
            "nothing chosen"
        );
        assert_eq!(
            asks.pending().len(),
            1,
            "a bad answer keeps the question open"
        );
        asks.answer(
            &id,
            Answer {
                choice: Some("b".into()),
                text: None,
            },
        )
        .unwrap();
        assert_eq!(
            h.await.unwrap().unwrap(),
            Outcome::Answered(Answer {
                choice: Some("b".into()),
                text: None
            })
        );
        assert!(asks.pending().is_empty());
        assert!(
            asks.answer(
                &id,
                Answer {
                    choice: Some("a".into()),
                    text: None
                }
            )
            .is_err(),
            "answered once"
        );
    }

    #[tokio::test]
    async fn confirm_text_decline_and_cancel() {
        let asks = Arc::new(Asks::default());
        let h = ask(&asks, "w1", json!({"kind": "confirm", "question": "Go?"})).await;
        let id = asks.pending()[0].id.clone();
        assert!(asks
            .answer(
                &id,
                Answer {
                    choice: Some("maybe".into()),
                    text: None
                }
            )
            .is_err());
        asks.answer(
            &id,
            Answer {
                choice: Some("yes".into()),
                text: None,
            },
        )
        .unwrap();
        assert_eq!(h.await.unwrap().unwrap().json()["choice"], "yes");

        let h = ask(&asks, "w2", json!({"kind": "text", "question": "Name?"})).await;
        let id = asks.pending()[0].id.clone();
        assert!(asks
            .answer(
                &id,
                Answer {
                    choice: None,
                    text: Some("  ".into())
                }
            )
            .is_err());
        asks.answer(
            &id,
            Answer {
                choice: None,
                text: Some("harnes".into()),
            },
        )
        .unwrap();
        assert_eq!(h.await.unwrap().unwrap().json()["text"], "harnes");

        let h = ask(&asks, "w3", json!({"kind": "confirm", "question": "Go?"})).await;
        assert!(asks.decline(&asks.pending()[0].id.clone()));
        assert_eq!(h.await.unwrap().unwrap(), Outcome::Declined);

        let h = ask(&asks, "w4", json!({"kind": "confirm", "question": "Go?"})).await;
        asks.cancel_agent("w4");
        assert_eq!(h.await.unwrap().unwrap(), Outcome::Cancelled);
    }

    #[tokio::test]
    async fn an_agent_has_one_question_at_a_time_and_silence_returns_the_default() {
        let asks = Arc::new(Asks::default());
        let a = asks.clone();
        let h = tokio::spawn(async move {
            a.request_for(
                "w1",
                &json!({"kind": "confirm", "question": "Go?", "default": "no"}),
                Some(Duration::from_millis(300)),
            )
            .await
        });
        while asks.pending().is_empty() {
            tokio::task::yield_now().await;
        }
        let second = asks
            .request("w1", &json!({"kind": "confirm", "question": "Again?"}))
            .await;
        assert!(second.unwrap_err().contains("déjà une question"));
        let out = h.await.unwrap().unwrap();
        assert_eq!(out, Outcome::TimedOut(Some("no".into())));
        assert_eq!(out.json()["default"], "no");
    }
}
