#![allow(dead_code)] // Shared by several test files; each uses only part of it.
//! The test seam: the whole app, in-process, on a fresh temporary database, with a controllable clock.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use jiff::tz::TimeZone;
use pebblepot::clock::TestClock;
use pebblepot::config::Config;
use scraper::{Html, Selector};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

pub const HOST: &str = "localhost:8070";

pub struct TestApp {
    pub router: Router,
    pub clock: TestClock,
    /// The app's shutdown token: cancelling it ends open event streams.
    pub shutdown: CancellationToken,
    data_dir: PathBuf,
    _tmp: tempfile::TempDir,
}

impl TestApp {
    pub async fn start() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("data");
        let clock = TestClock::at("2026-10-14T08:00:00Z".parse().unwrap());
        let shutdown = CancellationToken::new();
        let router = build(&data_dir, &clock, &shutdown).await;
        TestApp {
            router,
            clock,
            shutdown,
            data_dir,
            _tmp: tmp,
        }
    }

    /// Builds the app again on the same database, as a restart would.
    pub async fn restart(&mut self) {
        self.router = build(&self.data_dir, &self.clock, &self.shutdown).await;
    }

    pub async fn send(&self, request: Request<Body>) -> (StatusCode, String) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    pub async fn get(&self, path: &str) -> (StatusCode, String) {
        self.send(
            Request::get(path)
                .header("host", HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    /// A same-origin form POST, as htmx sends it from the board.
    pub async fn post_form(&self, path: &str, form: &str) -> (StatusCode, String) {
        self.send(
            Request::post(path)
                .header("host", HOST)
                .header("origin", format!("http://{HOST}"))
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form.to_string()))
                .unwrap(),
        )
        .await
    }

    /// Taps `member`'s face on a chore row. Returns the response fragment.
    pub async fn earn_chore(&self, member: &str, chore: &str) -> Fragment {
        let board = self.board().await;
        let member = board.member_id(member);
        let item = board.item_id(chore);
        let (status, body) = self
            .post_form(
                pebblepot::web::EARN_CHORE_PATH,
                &format!("member={member}&item={item}"),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        Fragment(Html::parse_fragment(&body))
    }

    /// Taps an Undo (a ✓, the toast's ↩︎ Undo or a today list ✕) for log entry `entry`.
    pub async fn undo(&self, entry: &str) -> Fragment {
        let (status, body) = self
            .post_form(pebblepot::web::UNDO_PATH, &format!("entry={entry}"))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        Fragment(Html::parse_fragment(&body))
    }

    /// Taps `member`'s pot: their today list.
    pub async fn today_list(&self, member: &str) -> TodayList {
        let member = self.board().await.member_id(member);
        let (status, body) = self
            .get(&format!("{}?member={member}", pebblepot::web::TODAY_PATH))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        TodayList(Html::parse_fragment(&body))
    }

    pub async fn pot(&self, member: &str) -> i64 {
        let board = self.board().await;
        board
            .pots()
            .into_iter()
            .find(|p| p.name == member)
            .unwrap_or_else(|| panic!("no pot for {member}"))
            .pebbles
    }

    pub async fn board(&self) -> BoardPage {
        let (status, body) = self.get("/").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        BoardPage(Html::parse_document(&body))
    }
}

async fn build(data_dir: &Path, clock: &TestClock, shutdown: &CancellationToken) -> Router {
    let config = Config {
        listen_addr: [127, 0, 0, 1].into(),
        port: 8070,
        data_dir: data_dir.to_path_buf(),
        time_zone: TimeZone::get("Europe/Rome").unwrap(),
    };
    pebblepot::build_app(&config, Arc::new(clock.clone()), shutdown.clone())
        .await
        .unwrap()
}

/// The board, read through the templates' semantic `data-*` markers only.
pub struct BoardPage(pub Html);

#[derive(Debug, PartialEq)]
pub struct Pot {
    pub name: String,
    pub pebbles: i64,
}

impl BoardPage {
    fn attrs(&self, selector: &str, attr: &str) -> Vec<String> {
        let sel = Selector::parse(selector).unwrap();
        self.0
            .select(&sel)
            .map(|e| e.value().attr(attr).unwrap().to_string())
            .collect()
    }

    pub fn pots(&self) -> Vec<Pot> {
        let sel = Selector::parse("[data-pots] [data-member]").unwrap();
        self.0
            .select(&sel)
            .map(|e| Pot {
                name: e.value().attr("data-member-name").unwrap().to_string(),
                pebbles: e.value().attr("data-pot").unwrap().parse().unwrap(),
            })
            .collect()
    }

    pub fn has_spend_button(&self) -> bool {
        !self
            .attrs("[data-pots] [data-spend]", "data-spend")
            .is_empty()
    }

    pub fn section_order(&self) -> Vec<String> {
        self.attrs("[data-section]", "data-section")
    }

    /// Item names in a section, in board order.
    pub fn items(&self, section: &str) -> Vec<String> {
        self.attrs(
            &format!("[data-section={section}] [data-item]"),
            "data-item-name",
        )
    }

    /// For every row in a section: the family member ids that have a face button on it.
    pub fn faces_per_row(&self, section: &str) -> Vec<Vec<String>> {
        let rows = Selector::parse(&format!("[data-section={section}] [data-item]")).unwrap();
        let faces = Selector::parse("[data-face-for]").unwrap();
        self.0
            .select(&rows)
            .map(|row| {
                row.select(&faces)
                    .map(|f| f.value().attr("data-face-for").unwrap().to_string())
                    .collect()
            })
            .collect()
    }

    pub fn member_ids(&self) -> Vec<String> {
        self.attrs("[data-pots] [data-member]", "data-member")
    }

    pub fn member_id(&self, name: &str) -> String {
        let sel = Selector::parse(&format!("[data-pots] [data-member-name=\"{name}\"]")).unwrap();
        let el = self
            .0
            .select(&sel)
            .next()
            .unwrap_or_else(|| panic!("no member {name}"));
        el.value().attr("data-member").unwrap().to_string()
    }

    pub fn item_id(&self, name: &str) -> String {
        let sel = Selector::parse(&format!("[data-item-name=\"{name}\"]")).unwrap();
        let el = self
            .0
            .select(&sel)
            .next()
            .unwrap_or_else(|| panic!("no item {name}"));
        el.value().attr("data-item").unwrap().to_string()
    }

    /// The log entry a ✓ undoes: `member`'s ✓ on this item's row.
    pub fn done_entry(&self, item: &str, member: &str) -> String {
        let member = self.member_id(member);
        let sel = Selector::parse(&format!(
            "[data-item-name=\"{item}\"] [data-face-for=\"{member}\"][data-done]"
        ))
        .unwrap();
        let el = self.0.select(&sel).next().expect("no ✓ to tap");
        el.value().attr("data-undo-entry").unwrap().to_string()
    }

    /// Names of the family members whose face shows a ✓ on this item's row.
    pub fn done_by(&self, item: &str) -> Vec<String> {
        let member_names: std::collections::HashMap<String, String> = self
            .pots()
            .into_iter()
            .map(|p| (self.member_id(&p.name), p.name))
            .collect();
        let sel = Selector::parse(&format!(
            "[data-item-name=\"{item}\"] [data-face-for][data-done]"
        ))
        .unwrap();
        self.0
            .select(&sel)
            .map(|f| member_names[f.value().attr("data-face-for").unwrap()].clone())
            .collect()
    }
}

/// A response fragment (a row plus out-of-band swaps), read through data-* markers.
pub struct Fragment(pub Html);

#[derive(Debug, PartialEq)]
pub struct Toast {
    pub kind: String,
    pub refusal: Option<String>,
    pub text: String,
}

impl Fragment {
    pub fn toast(&self) -> Toast {
        let sel = Selector::parse("[data-toast]").unwrap();
        let el = self.0.select(&sel).next().expect("no toast in response");
        Toast {
            kind: el.value().attr("data-toast").unwrap().to_string(),
            refusal: el.value().attr("data-refusal").map(str::to_string),
            text: el.text().collect::<String>().trim().to_string(),
        }
    }

    pub fn item_names(&self) -> Vec<String> {
        let sel = Selector::parse("[data-item]").unwrap();
        self.0
            .select(&sel)
            .map(|e| e.value().attr("data-item-name").unwrap().to_string())
            .collect()
    }
}

impl Fragment {
    /// The log entry the toast's ↩︎ Undo removes, if it offers one.
    pub fn toast_undo_entry(&self) -> Option<String> {
        let sel = Selector::parse("#toast [data-undo-entry]").unwrap();
        self.0
            .select(&sel)
            .next()
            .map(|e| e.value().attr("data-undo-entry").unwrap().to_string())
    }

    /// Family member ids whose face shows a ✓ on the row in this reply.
    pub fn done_faces(&self) -> Vec<String> {
        self.attrs("[data-item] [data-face-for][data-done]", "data-face-for")
    }

    /// The pots swapped in by this reply, by name.
    pub fn pots(&self) -> Vec<Pot> {
        BoardPage(self.0.clone()).pots()
    }

    /// The today list swapped in by this reply, if any.
    pub fn today_list(&self) -> Option<TodayList> {
        let sel = Selector::parse("[data-today-list]").unwrap();
        self.0
            .select(&sel)
            .next()
            .map(|e| TodayList(Html::parse_fragment(&e.html())))
    }

    fn attrs(&self, selector: &str, attr: &str) -> Vec<String> {
        let sel = Selector::parse(selector).unwrap();
        self.0
            .select(&sel)
            .map(|e| e.value().attr(attr).unwrap().to_string())
            .collect()
    }
}

/// A family member's today list, read through data-* markers.
pub struct TodayList(pub Html);

#[derive(Debug, PartialEq)]
pub struct TodayEntry {
    /// The log entry id its ✕ Undo removes.
    pub entry: String,
    pub name: String,
    pub amount: i64,
}

impl TodayList {
    pub fn entries(&self) -> Vec<TodayEntry> {
        let rows = Selector::parse("[data-entry]").unwrap();
        let undo = Selector::parse("[data-undo-entry]").unwrap();
        self.0
            .select(&rows)
            .map(|row| TodayEntry {
                entry: row
                    .select(&undo)
                    .next()
                    .expect("entry without ✕ Undo")
                    .value()
                    .attr("data-undo-entry")
                    .unwrap()
                    .to_string(),
                name: row.value().attr("data-entry-name").unwrap().to_string(),
                amount: row.value().attr("data-amount").unwrap().parse().unwrap(),
            })
            .collect()
    }

    pub fn names(&self) -> Vec<String> {
        self.entries().into_iter().map(|e| e.name).collect()
    }

    pub fn entry(&self, name: &str) -> String {
        self.entries()
            .into_iter()
            .find(|e| e.name == name)
            .unwrap_or_else(|| panic!("no {name} in today list"))
            .entry
    }
}
