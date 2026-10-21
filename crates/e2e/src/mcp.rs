//! MCP server: lets a Claude Code agent drive the real app and record the
//! resolution of natural-language `act` steps. The agent is the model; no
//! API key or extra model call is involved.

use crate::{
    app::AppProcess,
    browser::{find_browser, Session},
    cache::{self, Resolution},
    runner,
    spec::{self, Expect, Fill, Locator, Step},
};
use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Default)]
struct State {
    app: Option<AppProcess>,
    session: Option<Session>,
    recording: Vec<Step>,
    /// (file, step index) being resolved.
    target: Option<(String, usize)>,
}

pub struct Server {
    root: PathBuf,
    st: State,
}

fn tool(name: &str, desc: &str, props: Value, required: &[&str]) -> Value {
    json!({"name": name, "description": desc, "inputSchema": {"type": "object", "properties": props, "required": required}})
}

fn definitions() -> Value {
    let loc = json!({"type": "object", "description": "Locator: role, name, text, label, placeholder, testid, css, id (snapshot node like n12), exact, nth"});
    json!([
        tool("e2e_init", "Create harnes-e2e.yaml (app URL + start command) and tests/smoke.e2e.yaml if missing.", json!({"url": {"type": "string"}, "command": {"type": "string"}}), &["url"]),
        tool("e2e_run", "Run e2e specs (all of tests/**/*.e2e.yaml by default). Returns pass/fail and the act steps still to resolve.", json!({"files": {"type": "array", "items": {"type": "string"}}}), &[]),
        tool("e2e_open", "Open the app in a browser (starts it if needed) and return the screen snapshot.", json!({"path": {"type": "string", "default": "/"}}), &[]),
        tool("e2e_prepare", "Prepare to resolve an `act` step: replay the test up to that step, start recording, return the goal and the screen.", json!({"file": {"type": "string"}, "step": {"type": "integer", "description": "0-based step index"}}), &["file", "step"]),
        tool("e2e_observe", "Current screen as text, one node per line: #id role \"name\".", json!({}), &[]),
        tool("e2e_click", "Click an element. Recorded while resolving.", json!({"target": loc}), &["target"]),
        tool("e2e_fill", "Replace the text of an input. Recorded while resolving.", json!({"target": loc, "text": {"type": "string"}}), &["target", "text"]),
        tool("e2e_press", "Press a key on the focused element (Enter, Tab, Escape…). Recorded while resolving.", json!({"key": {"type": "string"}}), &["key"]),
        tool("e2e_goto", "Navigate to a path. Recorded while resolving.", json!({"path": {"type": "string"}}), &["path"]),
        tool("e2e_check", "Check an expectation without recording it: {target, text|contains|value|visible|count}.", json!({"expect": {"type": "object"}}), &["expect"]),
        tool("e2e_save", "Save the recorded actions as the resolution of the prepared act step, then verify the whole test.", json!({}), &[]),
        tool("e2e_screenshot", "Save a PNG of the current page.", json!({"path": {"type": "string"}}), &[]),
        tool("e2e_close", "Close the browser and stop the app if this server started it.", json!({}), &[])
    ])
}

impl Server {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            st: State::default(),
        }
    }

    async fn ensure_session(&mut self) -> Result<&Session> {
        if self.st.session.is_none() {
            let cfg = spec::load_config(&self.root)?;
            if self.st.app.is_none() {
                self.st.app = Some(AppProcess::start(&cfg.app, &self.root)?);
            }
            let exe = find_browser(cfg.browser.as_deref())?;
            self.st.session = Some(
                Session::launch(
                    &exe,
                    &cfg.app.url,
                    cfg.headed,
                    Duration::from_millis(cfg.timeout_ms),
                )
                .await?,
            );
        }
        Ok(self.st.session.as_ref().expect("session"))
    }

    async fn screen(&self) -> Result<String> {
        let s = self
            .st
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("pas de session : appelle e2e_open ou e2e_prepare"))?;
        Ok(s.snapshot().await?.render())
    }

    /// Replaces a snapshot-id locator by a stable one before recording it.
    async fn stable(&self, loc: &Locator) -> Result<Locator> {
        let Some(id) = &loc.id else {
            return Ok(loc.clone());
        };
        let s = self
            .st
            .session
            .as_ref()
            .ok_or_else(|| anyhow!("pas de session"))?;
        let snap = s.snapshot().await?;
        let n = snap
            .nodes
            .iter()
            .find(|n| &n.id == id)
            .ok_or_else(|| anyhow!("nœud {id} absent de l'écran"))?;
        Ok(match &n.testid {
            Some(t) => Locator {
                testid: Some(t.clone()),
                ..Default::default()
            },
            None => Locator {
                role: Some(n.role.clone()),
                name: Some(n.name.clone()),
                exact: true,
                ..Default::default()
            },
        })
    }

    async fn act(&mut self, step: Step) -> Result<String> {
        self.ensure_session().await?;
        let recorded = match &step {
            Step::Click(l) => Step::Click(self.stable(l).await?),
            Step::Fill(f) => Step::Fill(Fill {
                target: self.stable(&f.target).await?,
                text: f.text.clone(),
            }),
            other => other.clone(),
        };
        self.st
            .session
            .as_ref()
            .expect("session")
            .perform(&step)
            .await?;
        if self.st.target.is_some() {
            self.st.recording.push(recorded);
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.screen().await
    }

    async fn call(&mut self, name: &str, a: &Value) -> Result<String> {
        let s = |k: &str| {
            a.get(k)
                .and_then(Value::as_str)
                .map(String::from)
                .ok_or_else(|| anyhow!("argument manquant : {k}"))
        };
        let loc = |k: &str| -> Result<Locator> {
            Ok(serde_json::from_value(
                a.get(k)
                    .cloned()
                    .ok_or_else(|| anyhow!("argument manquant : {k}"))?,
            )?)
        };
        match name {
            "e2e_init" => {
                let w = crate::init(&self.root, &s("url")?, s("command").ok().as_deref())?;
                Ok(if w.is_empty() {
                    "déjà initialisé".into()
                } else {
                    format!("écrit : {}", w.join(", "))
                })
            }
            "e2e_run" => {
                let files = a
                    .get("files")
                    .and_then(Value::as_array)
                    .map(|v| {
                        v.iter()
                            .filter_map(Value::as_str)
                            .map(PathBuf::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let r = runner::run(
                    &self.root,
                    runner::RunOptions {
                        files,
                        headed: false,
                    },
                )
                .await;
                let mut out = r.describe();
                for u in &r.unresolved {
                    out.push_str(&format!(
                        "\nÀ résoudre : e2e_prepare {{file: \"{}\", step: {}}} — « {} » ({})",
                        u.file, u.step, u.act, u.reason
                    ));
                }
                Ok(out)
            }
            "e2e_open" => {
                let path = s("path").unwrap_or_else(|_| "/".into());
                self.ensure_session().await?;
                self.st
                    .session
                    .as_ref()
                    .expect("session")
                    .perform(&Step::Open(path))
                    .await?;
                self.screen().await
            }
            "e2e_prepare" => {
                let file = s("file")?;
                let step = a
                    .get("step")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| anyhow!("argument manquant : step"))?
                    as usize;
                let (_, test) = runner::load(&self.root, &file)?;
                let Some(Step::Act(goal)) = test.steps.get(step).cloned() else {
                    bail!("l'étape {step} de {file} n'est pas une étape `act`")
                };
                if let Some(old) = self.st.session.take() {
                    old.close().await;
                }
                self.ensure_session().await?;
                let session = self.st.session.as_ref().expect("session");
                runner::run_steps(&self.root, session, &file, &test, Some(step))
                    .await
                    .map_err(|(i, m, _)| anyhow!("étape {i} avant l'act a échoué : {m}"))?;
                self.st.recording.clear();
                self.st.target = Some((file, step));
                Ok(format!("Objectif à réaliser : « {goal} »\nRéalise-le avec e2e_click / e2e_fill / e2e_press / e2e_goto, vérifie avec e2e_check, puis e2e_save.\n\n{}", self.screen().await?))
            }
            "e2e_observe" => self.screen().await,
            "e2e_click" => self.act(Step::Click(loc("target")?)).await,
            "e2e_fill" => {
                self.act(Step::Fill(Fill {
                    target: loc("target")?,
                    text: s("text")?,
                }))
                .await
            }
            "e2e_press" => self.act(Step::Press(s("key")?)).await,
            "e2e_goto" => self.act(Step::Open(s("path")?)).await,
            "e2e_check" => {
                let e: Expect =
                    serde_json::from_value(a.get("expect").cloned().unwrap_or_default())?;
                self.ensure_session().await?.check(&e).await?;
                Ok("OK".into())
            }
            "e2e_save" => {
                let (file, step) =
                    self.st.target.clone().ok_or_else(|| {
                        anyhow!("rien à enregistrer : appelle d'abord e2e_prepare")
                    })?;
                let (_, test) = runner::load(&self.root, &file)?;
                let Some(Step::Act(goal)) = test.steps.get(step).cloned() else {
                    bail!("étape {step} introuvable")
                };
                let key = cache::key(&file, &test.name, step, &goal);
                let res = Resolution {
                    test: test.name.clone(),
                    act: goal,
                    actions: self.st.recording.clone(),
                    recorded_at: cache::now(),
                };
                let p = cache::put(&self.root, &key, &res)?;
                self.st.target = None;
                let r = runner::run(
                    &self.root,
                    runner::RunOptions {
                        files: vec![PathBuf::from(&file)],
                        headed: false,
                    },
                )
                .await;
                Ok(format!(
                    "{} action(s) enregistrée(s) dans {}\nVérification : {}",
                    res.actions.len(),
                    p.display(),
                    r.describe()
                ))
            }
            "e2e_screenshot" => {
                let p = self
                    .root
                    .join(s("path").unwrap_or_else(|_| ".harnes/e2e/screenshot.png".into()));
                self.ensure_session().await?.screenshot(&p).await?;
                Ok(format!("capture : {}", p.display()))
            }
            "e2e_close" => {
                if let Some(sess) = self.st.session.take() {
                    sess.close().await;
                }
                if let Some(mut app) = self.st.app.take() {
                    app.stop();
                }
                Ok("fermé".into())
            }
            other => bail!("outil inconnu : {other}"),
        }
    }

    pub async fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned()?;
        let method = msg["method"].as_str().unwrap_or("");
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": msg.pointer("/params/protocolVersion").and_then(Value::as_str).unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "harnes-e2e", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Write tests as tests/**/*.e2e.yaml. For each `act` step reported unresolved by e2e_run: e2e_prepare, perform the goal with e2e_click/e2e_fill/e2e_press, verify with e2e_check, then e2e_save."
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": definitions()})),
            "tools/call" => {
                let name = msg
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let args = msg
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or(json!({}));
                Ok(match self.call(&name, &args).await {
                    Ok(t) => json!({"content": [{"type": "text", "text": t}]}),
                    Err(e) => {
                        json!({"content": [{"type": "text", "text": format!("erreur : {e:#}")}], "isError": true})
                    }
                })
            }
            _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        })
    }

    pub async fn serve(mut self) -> Result<()> {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        let mut out = tokio::io::stdout();
        while let Some(line) = lines.next_line().await? {
            if line.trim().is_empty() {
                continue;
            }
            let reply = match serde_json::from_str::<Value>(&line) {
                Ok(m) => self.handle(&m).await,
                Err(e) => Some(
                    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}}),
                ),
            };
            if let Some(r) = reply {
                out.write_all(format!("{r}\n").as_bytes()).await?;
                out.flush().await?;
            }
        }
        let _ = self.call("e2e_close", &json!({})).await;
        Ok(())
    }
}
