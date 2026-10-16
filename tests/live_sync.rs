mod common;

use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::http::{Request, StatusCode, header};
use common::{BoardPage, HOST, TestApp};
use http_body_util::BodyExt;
use pebblepot::web::{BOARD_REFRESH_PATH, EVENTS_PATH};
use scraper::Html;
use tower::ServiceExt;

/// An open live-update stream, read as raw SSE text.
struct Events {
    body: Body,
    buffer: String,
}

impl Events {
    async fn open(app: &TestApp) -> Self {
        let response = app
            .router
            .clone()
            .oneshot(
                Request::get(EVENTS_PATH)
                    .header("host", HOST)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );
        Events {
            body: response.into_body(),
            buffer: String::new(),
        }
    }

    /// The next event's (name, data), or `None` if nothing arrives in time.
    async fn next_event(&mut self, wait: Duration) -> Option<(String, String)> {
        tokio::time::timeout(wait, async {
            loop {
                if let Some(end) = self.buffer.find("\n\n") {
                    let block: String = self.buffer.drain(..end + 2).collect();
                    if let Some(event) = parse_event(&block) {
                        return event;
                    }
                    continue; // a keep-alive comment
                }
                let chunk = self.next_chunk().await.expect("stream ended");
                self.buffer.push_str(&String::from_utf8_lossy(&chunk));
            }
        })
        .await
        .ok()
    }

    async fn next_chunk(&mut self) -> Option<Bytes> {
        loop {
            let frame = self.body.frame().await?.unwrap();
            if let Ok(data) = frame.into_data() {
                return Some(data);
            }
        }
    }
}

fn parse_event(block: &str) -> Option<(String, String)> {
    let mut name = None;
    let mut data = Vec::new();
    for line in block.lines() {
        if let Some(v) = line.strip_prefix("event:") {
            name = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("data:") {
            data.push(v.trim().to_string());
        }
    }
    name.map(|n| (n, data.join("\n")))
}

const SOON: Duration = Duration::from_secs(2);
const QUIET: Duration = Duration::from_millis(300);

#[tokio::test]
async fn an_earn_sends_a_changed_event_with_the_version() {
    let app = TestApp::start().await;
    let mut events = Events::open(&app).await;

    app.earn_chore("Kid 1", "Make bed").await;

    let (name, data) = events.next_event(SOON).await.expect("no event");
    assert_eq!(name, "changed");
    assert!(!data.is_empty(), "SSE drops events with empty data");
    assert_eq!(data, "1");

    app.earn_chore("Kid 2", "Make bed").await;
    assert_eq!(
        events.next_event(SOON).await,
        Some(("changed".into(), "2".into()))
    );
}

#[tokio::test]
async fn every_open_stream_hears_about_a_change() {
    let app = TestApp::start().await;
    let mut ipad = Events::open(&app).await;
    let mut phone = Events::open(&app).await;

    app.earn_chore("Mom", "Rake leaves").await;

    assert_eq!(ipad.next_event(SOON).await.unwrap().0, "changed");
    assert_eq!(phone.next_event(SOON).await.unwrap().0, "changed");
}

#[tokio::test]
async fn reading_the_board_sends_no_event() {
    let app = TestApp::start().await;
    let mut events = Events::open(&app).await;

    app.board().await;
    app.get(&format!("{BOARD_REFRESH_PATH}?q=wash")).await;

    assert_eq!(events.next_event(QUIET).await, None);
}

#[tokio::test]
async fn a_rejected_cross_site_post_sends_no_event() {
    let app = TestApp::start().await;
    let mut events = Events::open(&app).await;

    let (status, _) = app
        .send(
            Request::post(pebblepot::web::EARN_CHORE_PATH)
                .header("host", HOST)
                .header("origin", "http://evil.example")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from("member=1&item=1"))
                .unwrap(),
        )
        .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(events.next_event(QUIET).await, None);
}

#[tokio::test]
async fn the_board_refresh_brings_current_pots_and_ticks() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Tidy toys").await;

    let (status, body) = app.get(BOARD_REFRESH_PATH).await;
    assert_eq!(status, StatusCode::OK);
    let refresh = BoardPage(Html::parse_document(&body));

    let kid2 = refresh.pots().into_iter().find(|p| p.name == "Kid 2");
    assert_eq!(kid2.unwrap().pebbles, 2);
    assert_eq!(refresh.done_by("Tidy toys"), ["Kid 2"]);
    assert!(body.contains("hx-swap-oob"), "swapped in out of band");
}

#[tokio::test]
async fn the_board_refresh_keeps_the_other_jobs_search() {
    let app = TestApp::start().await;
    let page = app.board().await;
    let all_other_jobs = page.items("occasional_chores");

    let (_, body) = app.get(&format!("{BOARD_REFRESH_PATH}?q=LEAVES")).await;
    let refresh = BoardPage(Html::parse_document(&body));

    assert_eq!(rows(&refresh, "occasional_chores"), ["Rake leaves"]);
    assert!(all_other_jobs.len() > 1);
    // The other sections are never filtered.
    assert_eq!(rows(&refresh, "daily_chores"), page.items("daily_chores"));
}

/// Item names in a section's rows container (the refresh has no section headers).
fn rows(page: &BoardPage, section: &str) -> Vec<String> {
    let sel = scraper::Selector::parse(&format!("#rows-{section} [data-item]")).unwrap();
    page.0
        .select(&sel)
        .map(|e| e.value().attr("data-item-name").unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn shutting_down_ends_open_streams_at_once() {
    let app = TestApp::start().await;
    let mut events = Events::open(&app).await;

    app.shutdown.cancel();

    let ended = tokio::time::timeout(Duration::from_millis(500), events.next_chunk()).await;
    assert_eq!(ended, Ok(None), "the stream should end, not linger");
}

#[tokio::test]
async fn an_undo_also_reaches_every_open_board() {
    // Undo (ticket 03) needs no live-sync code of its own: the write middleware covers it.
    let app = TestApp::start().await;
    let earned = app.earn_chore("Kid 1", "Make bed").await;
    let entry = earned.toast_undo_entry().expect("earn toast offers undo");
    let mut events = Events::open(&app).await;

    app.undo(&entry).await;

    assert_eq!(
        events.next_event(SOON).await,
        Some(("changed".into(), "2".into()))
    );
    assert_eq!(app.pot("Kid 1").await, 0);
}
