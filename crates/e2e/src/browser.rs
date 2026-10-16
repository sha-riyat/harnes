//! Browser session over the Chrome DevTools Protocol (Chrome, Edge or any Chromium).

use crate::spec::{Expect, Locator, Step};
use anyhow::{anyhow, bail, Context, Result};
use chromiumoxide::{Browser, BrowserConfig, Page};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

const PAGE_JS: &str = include_str!("page.js");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub states: Vec<String>,
    pub testid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub url: String,
    pub title: String,
    pub nodes: Vec<Node>,
}

impl Snapshot {
    /// One node per line: `#n3 status "0" value="…" [focused]`.
    pub fn render(&self) -> String {
        let mut s = format!("url: {}\ntitle: {}\n", self.url, self.title);
        for n in &self.nodes {
            s.push_str(&format!("#{} {} \"{}\"", n.id, n.role, n.name));
            if let Some(v) = n.value.as_ref().filter(|v| !v.is_empty()) {
                s.push_str(&format!(" value=\"{v}\""));
            }
            if let Some(t) = &n.testid {
                s.push_str(&format!(" testid={t}"));
            }
            if !n.states.is_empty() {
                s.push_str(&format!(" [{}]", n.states.join(", ")));
            }
            s.push('\n');
        }
        s
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Found {
    pub count: usize,
    pub text: Option<String>,
    pub value: Option<String>,
    #[allow(dead_code)]
    pub id: Option<String>,
}

/// Finds a Chromium-family browser on this machine.
pub fn find_browser(configured: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = configured {
        return Ok(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("HARNES_BROWSER") {
        return Ok(PathBuf::from(p));
    }
    let candidates: &[&str] = if cfg!(windows) {
        &[
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"]
    } else {
        &["/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser", "/usr/bin/microsoft-edge"]
    };
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .ok_or_else(|| anyhow!("aucun navigateur Chromium trouvé : définis `browser` dans harnes-e2e.yaml ou HARNES_BROWSER"))
}

pub struct Session {
    browser: Browser,
    pub page: Page,
    handler: JoinHandle<()>,
    pub base_url: String,
    pub timeout: Duration,
}

impl Session {
    pub async fn launch(exe: &Path, base_url: &str, headed: bool, timeout: Duration) -> Result<Self> {
        let profile = std::env::temp_dir().join(format!("harnes-e2e-{}-{}", std::process::id(), rand_suffix()));
        let mut cfg = BrowserConfig::builder()
            .chrome_executable(exe)
            .user_data_dir(&profile)
            .window_size(1280, 800)
            .arg("--no-first-run")
            .arg("--disable-extensions");
        if headed {
            cfg = cfg.with_head();
        }
        let (browser, mut handler) = Browser::launch(cfg.build().map_err(|e| anyhow!(e))?)
            .await
            .with_context(|| format!("lancement de {}", exe.display()))?;
        let handler = tokio::spawn(async move { while handler.next().await.is_some() {} });
        let page = browser.new_page("about:blank").await?;
        Ok(Self { browser, page, handler, base_url: base_url.trim_end_matches('/').to_string(), timeout })
    }

    pub fn url_for(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}/{}", self.base_url, path.trim_start_matches('/'))
        }
    }

    async fn js<T: for<'de> Deserialize<'de>>(&self, expr: &str) -> Result<T> {
        let code = format!("{PAGE_JS}\n{expr}");
        let v = self.page.evaluate(code).await.map_err(|e| anyhow!("évaluation JS : {e}"))?;
        v.into_value::<T>().map_err(|e| anyhow!("résultat JS : {e}"))
    }

    pub async fn snapshot(&self) -> Result<Snapshot> {
        self.js("window.__h.snapshot()").await
    }

    pub async fn query(&self, loc: &Locator) -> Result<Found> {
        let q = serde_json::to_string(loc)?;
        self.js(&format!("window.__h.query({q})")).await
    }

    /// Polls until the locator matches at least one element.
    async fn wait_for(&self, loc: &Locator) -> Result<Found> {
        let start = Instant::now();
        loop {
            if let Ok(f) = self.query(loc).await {
                if f.count > (loc.nth.unwrap_or(0)) {
                    return Ok(f);
                }
            }
            if start.elapsed() > self.timeout {
                bail!("élément introuvable : {}", loc.describe());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn target(&self) -> Result<chromiumoxide::Element> {
        Ok(self.page.find_element("[data-harnes-target='1']").await?)
    }

    /// Executes one primitive step (open, click, fill, press, wait).
    pub async fn perform(&self, step: &Step) -> Result<()> {
        match step {
            Step::Open(path) => {
                self.page.goto(self.url_for(path)).await?;
                self.page.wait_for_navigation().await?;
            }
            Step::Click(loc) => {
                self.wait_for(loc).await?;
                let el = self.target().await?;
                el.scroll_into_view().await?;
                el.click().await?;
            }
            Step::Fill(f) => {
                self.wait_for(&f.target).await?;
                self.js::<bool>("(() => { const el = document.querySelector('[data-harnes-target]'); el.focus(); if ('value' in el) { el.value = ''; el.dispatchEvent(new Event('input', {bubbles: true})); } return true; })()").await?;
                self.target().await?.type_str(&f.text).await?;
            }
            Step::Press(key) => {
                self.js::<bool>("window.__h.markFocused()").await?;
                self.target().await?.press_key(key).await?;
            }
            Step::Wait(ms) => tokio::time::sleep(Duration::from_millis(*ms)).await,
            Step::Expect(_) | Step::Act(_) => bail!("pas une action primitive"),
        }
        Ok(())
    }

    /// Polls until the expectation holds or the timeout expires.
    pub async fn check(&self, e: &Expect) -> Result<()> {
        let start = Instant::now();
        let mut last;
        loop {
            match self.query(&e.target).await {
                Ok(f) => match verdict(e, &f) {
                    None => return Ok(()),
                    Some(why) => last = why,
                },
                Err(err) => last = err.to_string(),
            }
            if start.elapsed() > self.timeout {
                bail!("attente non satisfaite {} : {last}", e.target.describe());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    pub async fn screenshot(&self, path: &Path) -> Result<()> {
        let png = self.page.screenshot(chromiumoxide::page::ScreenshotParams::builder().build()).await?;
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(path, png)?;
        Ok(())
    }

    pub async fn close(mut self) {
        let _ = self.browser.close().await;
        let _ = self.browser.wait().await;
        self.handler.abort();
    }
}

/// `None` when the expectation holds, otherwise why not.
fn verdict(e: &Expect, f: &Found) -> Option<String> {
    let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(n) = e.count {
        return (f.count != n).then(|| format!("{} élément(s) au lieu de {n}", f.count));
    }
    if e.visible == Some(false) {
        return (f.count > 0).then(|| "l'élément est encore visible".to_string());
    }
    if f.count == 0 {
        return Some("aucun élément".into());
    }
    let text = f.text.clone().unwrap_or_default();
    if let Some(t) = &e.text {
        if norm(&text) != norm(t) {
            return Some(format!("texte \"{text}\" au lieu de \"{t}\""));
        }
    }
    if let Some(c) = &e.contains {
        if !text.to_lowercase().contains(&c.to_lowercase()) {
            return Some(format!("texte \"{text}\" ne contient pas \"{c}\""));
        }
    }
    if let Some(v) = &e.value {
        let got = f.value.clone().unwrap_or_default();
        if &got != v {
            return Some(format!("valeur \"{got}\" au lieu de \"{v}\""));
        }
    }
    None
}

fn rand_suffix() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
}
