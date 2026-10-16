//! HTTP: routes, templates and static assets.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use askama::Template;
use axum::Router;
use axum::extract::{Form, Query, Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use futures_util::{Stream, StreamExt};
use jiff::tz::TimeZone;
use serde::Deserialize;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use tokio_util::sync::CancellationToken;
use tower_http::csrf::CsrfLayer;

use crate::board::{self, Board, Item, TodayList};
use crate::clock::{Clock, local_day};
use crate::rules::{self, CommandError, Refusal};
use crate::store::Store;

/// Versioned so it can be cached for good.
pub const HTMX_PATH: &str = "/static/htmx-2.0.11.min.js";
const HTMX_JS: &str = include_str!("../static/htmx-2.0.11.min.js");
pub const CSS_PATH: &str = "/static/app.css";
const APP_CSS: &str = include_str!("../static/app.css");
pub const SSE_EXT_PATH: &str = "/static/htmx-ext-sse-2.2.4.min.js";
const SSE_EXT_JS: &str = include_str!("../static/htmx-ext-sse-2.2.4.min.js");
pub const LIVE_JS_PATH: &str = "/static/live.js";
const LIVE_JS: &str = include_str!("../static/live.js");

/// Pebble colours, cycled through when drawing dots.
const PEBBLE_COLOURS: [&str; 8] = [
    "#f4a261", "#e76f51", "#8ecae6", "#90be6d", "#c77dff", "#f9c74f", "#ff8fab", "#76c7c0",
];
/// A pot card draws at most this many dots; the number tells the rest.
const MAX_POT_DOTS: i64 = 20;

#[derive(Clone)]
pub struct AppState {
    pub store: Store,
    pub clock: Arc<dyn Clock>,
    pub time_zone: TimeZone,
    /// Live sync: the board's version, bumped after every successful write (see `bump_version_after_writes`).
    pub version: watch::Sender<u64>,
    /// Cancelled on shutdown; ends every open event stream so the server can exit at once.
    pub shutdown: CancellationToken,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(board_page))
        .route(OCCASIONAL_ROWS_PATH, get(occasional_rows))
        .route(EARN_CHORE_PATH, post(earn_chore))
        .route(EARN_GOOD_DEED_PATH, post(earn_good_deed))
        .route(TODAY_PATH, get(today_list))
        .route(UNDO_PATH, post(undo))
        .route(SPEND_PATH, get(spend_sheet))
        .route(REDEEM_PATH, post(redeem))
        .route(GOAL_SET_PATH, post(set_goal))
        .route(GOAL_GIVE_UP_PATH, post(give_up_goal))
        .route(GOAL_GOT_IT_PATH, post(goal_got_it))
        .route(EVENTS_PATH, get(events))
        .route(BOARD_REFRESH_PATH, get(board_refresh))
        .route(
            SSE_EXT_PATH,
            get(|| async {
                asset(
                    SSE_EXT_JS,
                    "text/javascript",
                    "public, max-age=31536000, immutable",
                )
            }),
        )
        .route(
            LIVE_JS_PATH,
            get(|| async { asset(LIVE_JS, "text/javascript", "no-cache") }),
        )
        .route(
            HTMX_PATH,
            get(|| async {
                asset(
                    HTMX_JS,
                    "text/javascript",
                    "public, max-age=31536000, immutable",
                )
            }),
        )
        .route(
            CSS_PATH,
            get(|| async { asset(APP_CSS, "text/css", "no-cache") }),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            bump_version_after_writes,
        ))
        .layer(CsrfLayer::new())
        .with_state(state)
}

/// Live sync, done once for every route: after any request that may change state (any method but
/// GET, HEAD or OPTIONS) answers 2xx, bump the version so every open board refetches. Handlers commit
/// before they answer, so a refetch never sees uncommitted state. New command endpoints are covered
/// without code of their own. A refused command (200 with a refusal toast) bumps too: harmless.
async fn bump_version_after_writes(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let may_write = !matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let response = next.run(request).await;
    if may_write && response.status().is_success() {
        state.version.send_modify(|v| *v += 1);
    }
    response
}

/// The one live-update stream: a `changed` event carrying the new version after each write.
/// Bursts coalesce (only the latest version is sent). The data must be non-empty, because
/// browsers drop SSE events with empty data. Ends when the server shuts down.
async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let changes = WatchStream::from_changes(state.version.subscribe())
        .map(|version| Ok(Event::default().event("changed").data(version.to_string())))
        .take_until(state.shutdown.clone().cancelled_owned());
    Sse::new(changes).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

fn asset(body: &'static str, content_type: &'static str, cache: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, cache),
        ],
        body,
    )
        .into_response()
}

struct AppError(crate::store::Error);

impl From<crate::store::Error> for AppError {
    fn from(e: crate::store::Error) -> Self {
        AppError(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("error: {}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong").into_response()
    }
}

fn dots(n: i64) -> Vec<&'static str> {
    (0..n.max(0) as usize)
        .map(|i| PEBBLE_COLOURS[i % PEBBLE_COLOURS.len()])
        .collect()
}

/// Which board section an item belongs to, and how a tap on it earns.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    DailyChores,
    GoodDeeds,
    OccasionalChores,
}

impl Section {
    fn key(self) -> &'static str {
        match self {
            Section::DailyChores => "daily_chores",
            Section::GoodDeeds => "good_deeds",
            Section::OccasionalChores => "occasional_chores",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Section::DailyChores => "☀️ Today's jobs",
            Section::GoodDeeds => "💛 Kind things",
            Section::OccasionalChores => "🧺 Other jobs",
        }
    }

    /// Where a face tap posts to; `None` while the section is still read-only.
    fn earn_path(self) -> Option<&'static str> {
        match self {
            Section::DailyChores | Section::OccasionalChores => Some(EARN_CHORE_PATH),
            Section::GoodDeeds => Some(EARN_GOOD_DEED_PATH),
        }
    }

    fn is_chores(self) -> bool {
        matches!(self, Section::DailyChores | Section::OccasionalChores)
    }
}

pub const EARN_CHORE_PATH: &str = "/earn/chore";
/// POST `member=<id>&item=<id>`: a face tap on a 💛 Kind things row.
pub const EARN_GOOD_DEED_PATH: &str = "/earn/good-deed";
pub const OCCASIONAL_ROWS_PATH: &str = "/occasional";
pub const EVENTS_PATH: &str = "/events";
/// Everything a live refetch replaces: pots and section rows, swapped in out of band.
pub const BOARD_REFRESH_PATH: &str = "/board";
/// GET `?member=<id>`: that family member's today list, as a sheet.
pub const TODAY_PATH: &str = "/today";
/// POST `entry=<id>` (and `item=<id>` from a ✓, to refresh its row even if refused).
pub const UNDO_PATH: &str = "/undo";
/// GET, optionally `?member=<id>`: the Spend sheet, asking whose pebbles, then that person's rewards.
pub const SPEND_PATH: &str = "/spend";
/// POST `member=<id>&item=<id>`: redeem a reward from that person's pot.
pub const REDEEM_PATH: &str = "/redeem";
/// POST `member=<id>&item=<id>`: ⭐ Save for this, setting that reward as the person's goal.
pub const GOAL_SET_PATH: &str = "/goal/set";
/// POST `member=<id>`: give up that person's goal, refunding its price.
pub const GOAL_GIVE_UP_PATH: &str = "/goal/give-up";
/// POST `member=<id>`: Got it, the Ready goal was handed over.
pub const GOAL_GOT_IT_PATH: &str = "/goal/got-it";

struct MemberView {
    id: i64,
    name: String,
    face: String,
    colour: String,
    pot: i64,
    pot_dots: Vec<&'static str>,
    /// The goal this person is saving for: their pot card shows it instead of pebbles.
    goal: Option<GoalView>,
}

/// A goal as the pot card and the Spend sheet show it.
struct GoalView {
    name: String,
    picture: String,
    /// pot + price: what can be spent while saving.
    saved: i64,
    /// Pebbles until Ready: how far the pot is below zero.
    to_go: i64,
    ready: bool,
    /// How much of the picture is in colour: saved ÷ price, in whole percent (0 to 100).
    fill: i64,
}

fn goal_view(goal: &board::Goal, pot: i64) -> GoalView {
    let saved = goal.saved(pot);
    GoalView {
        name: goal.name.clone(),
        picture: goal.picture.clone(),
        saved,
        to_go: (-pot).max(0),
        ready: goal.is_ready(pot),
        fill: if goal.price > 0 {
            (saved * 100 / goal.price).clamp(0, 100)
        } else {
            100
        },
    }
}

struct FaceView {
    member_id: i64,
    name: String,
    face: String,
    colour: String,
    /// Chore already earned by this person today: shown as a ✓ that undoes this log entry.
    done_entry: Option<i64>,
    /// Good deed earned by this person today: the count badge, which undoes the most recent.
    badge: Option<board::GoodDeedCount>,
}

struct ItemView {
    id: i64,
    name: String,
    picture: String,
    value: i64,
    dots: Vec<&'static str>,
    earn_path: Option<&'static str>,
    faces: Vec<FaceView>,
    /// Swapped in out of band (by id) rather than as the reply's target.
    oob: bool,
}

struct SectionView {
    key: &'static str,
    title: &'static str,
    searchable: bool,
    items: Vec<ItemView>,
}

struct ToastView {
    /// "earned", "undone" or "refused".
    kind: &'static str,
    refusal: &'static str,
    text: String,
    /// The log entry this toast reported, offered for undo.
    undo_entry: Option<i64>,
}

impl ToastView {
    fn refused(refusal: Refusal) -> Self {
        ToastView {
            kind: "refused",
            refusal: refusal.code(),
            text: refusal.message().to_string(),
            undo_entry: None,
        }
    }
}

struct TodayEntryView {
    id: i64,
    name: String,
    picture: String,
    /// With its sign: "+2", "-5".
    amount: String,
    raw_amount: i64,
}

struct TodayListView {
    member: MemberView,
    entries: Vec<TodayEntryView>,
}

fn member_view(m: &board::Member) -> MemberView {
    MemberView {
        pot_dots: dots(m.pot.min(MAX_POT_DOTS)),
        id: m.id,
        name: m.name.clone(),
        face: m.face.clone(),
        colour: m.colour.clone(),
        pot: m.pot,
        goal: None,
    }
}

/// A family member as the board shows them, with their goal if they're saving for one.
fn board_member_view(board: &Board, m: &board::Member) -> MemberView {
    MemberView {
        goal: board.goals.get(&m.id).map(|g| goal_view(g, m.pot)),
        ..member_view(m)
    }
}

fn member_views(board: &Board) -> Vec<MemberView> {
    board
        .members
        .iter()
        .map(|m| board_member_view(board, m))
        .collect()
}

fn today_list_view(list: &TodayList) -> TodayListView {
    TodayListView {
        member: member_view(&list.member),
        entries: list
            .entries
            .iter()
            .map(|e| TodayEntryView {
                id: e.id,
                name: e.name.clone(),
                picture: e.picture.clone(),
                amount: format!("{:+}", e.amount),
                raw_amount: e.amount,
            })
            .collect(),
    }
}

fn item_view(board: &Board, section: Section, item: &Item) -> ItemView {
    ItemView {
        id: item.id,
        name: item.name.clone(),
        picture: item.picture.clone(),
        value: item.value,
        dots: dots(item.value),
        earn_path: section.earn_path(),
        faces: board
            .members
            .iter()
            .map(|m| FaceView {
                member_id: m.id,
                name: m.name.clone(),
                face: m.face.clone(),
                colour: m.colour.clone(),
                done_entry: if section.is_chores() {
                    board.done.get(&(m.id, item.id)).copied()
                } else {
                    None
                },
                badge: if section == Section::GoodDeeds {
                    board.good_deed_counts.get(&(m.id, item.id)).copied()
                } else {
                    None
                },
            })
            .collect(),
        oob: false,
    }
}

fn section_items(board: &Board, section: Section) -> &[Item] {
    match section {
        Section::DailyChores => &board.daily_chores,
        Section::GoodDeeds => &board.good_deeds,
        Section::OccasionalChores => &board.occasional_chores,
    }
}

/// Case-insensitive name filter for the Other jobs search box.
fn matches_search(item: &Item, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty() || item.name.to_lowercase().contains(&query)
}

#[derive(Template)]
#[template(path = "board.html")]
struct BoardPage {
    htmx_path: &'static str,
    css_path: &'static str,
    sse_ext_path: &'static str,
    live_js_path: &'static str,
    events_path: &'static str,
    board_refresh_path: &'static str,
    occasional_rows_path: &'static str,
    members: Vec<MemberView>,
    sections: Vec<SectionView>,
    oob: bool,
}

#[derive(Template)]
#[template(path = "rows.html")]
struct RowsPartial {
    items: Vec<ItemView>,
}

/// A family member's today list, opened by tapping their pot.
#[derive(Template)]
#[template(path = "today.html")]
struct TodayPartial {
    today: TodayListView,
    oob: bool,
}

/// The reply to an undo: everything out of band, since the tap may come from a ✓, the toast or
/// the today list. The affected row and today list are only swapped in if they're on screen.
#[derive(Template)]
#[template(path = "undo_response.html")]
struct UndoResponse {
    item: Option<ItemView>,
    members: Vec<MemberView>,
    today: Option<TodayListView>,
    toast: ToastView,
    oob: bool,
}

/// The reply to a face tap: the tapped row, plus the pots and a toast swapped in out of band.
#[derive(Template)]
#[template(path = "earn_response.html")]
struct EarnResponse {
    item: ItemView,
    members: Vec<MemberView>,
    toast: ToastView,
    oob: bool,
}

impl AppState {
    fn today(&self) -> jiff::civil::Date {
        local_day(self.clock.now(), &self.time_zone)
    }
}

fn render(t: impl Template) -> Result<Html<String>, AppError> {
    t.render().map(Html).map_err(|e| AppError(e.into()))
}

async fn board_page(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let board = board::load(&state.store, state.today()).await?;
    let sections = [
        Section::DailyChores,
        Section::GoodDeeds,
        Section::OccasionalChores,
    ]
    .into_iter()
    .map(|section| SectionView {
        key: section.key(),
        title: section.title(),
        searchable: section == Section::OccasionalChores,
        items: section_items(&board, section)
            .iter()
            .map(|item| item_view(&board, section, item))
            .collect(),
    })
    .collect();
    render(BoardPage {
        htmx_path: HTMX_PATH,
        css_path: CSS_PATH,
        sse_ext_path: SSE_EXT_PATH,
        live_js_path: LIVE_JS_PATH,
        events_path: EVENTS_PATH,
        board_refresh_path: BOARD_REFRESH_PATH,
        occasional_rows_path: OCCASIONAL_ROWS_PATH,
        members: member_views(&board),
        sections,
        oob: false,
    })
}

#[derive(Deserialize)]
struct SearchQuery {
    #[serde(default)]
    q: String,
}

async fn occasional_rows(
    State(state): State<AppState>,
    Query(search): Query<SearchQuery>,
) -> Result<Html<String>, AppError> {
    let board = board::load(&state.store, state.today()).await?;
    let items = board
        .occasional_chores
        .iter()
        .filter(|item| matches_search(item, &search.q))
        .map(|item| item_view(&board, Section::OccasionalChores, item))
        .collect();
    render(RowsPartial { items })
}

#[derive(Deserialize)]
struct EarnForm {
    member: i64,
    item: i64,
}

async fn earn_chore(
    State(state): State<AppState>,
    Form(form): Form<EarnForm>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let result =
        rules::earn_chore(&state.store, now, &state.time_zone, form.member, form.item).await;
    let toast = match result {
        Ok(earned) => ToastView {
            kind: "earned",
            refusal: "",
            text: format!("{} {} +{}", earned.face, earned.picture, earned.amount),
            undo_entry: Some(earned.entry_id),
        },
        Err(CommandError::Refused(refusal)) => ToastView::refused(refusal),
        Err(CommandError::Store(e)) => return Err(AppError(e)),
    };
    let board = board::load(&state.store, local_day(now, &state.time_zone)).await?;
    let found = [Section::DailyChores, Section::OccasionalChores]
        .into_iter()
        .find_map(|section| {
            section_items(&board, section)
                .iter()
                .find(|item| item.id == form.item)
                .map(|item| item_view(&board, section, item))
        });
    let Some(item) = found else {
        return Ok((StatusCode::NOT_FOUND, toast.text).into_response());
    };
    Ok(render(EarnResponse {
        item,
        members: member_views(&board),
        toast,
        oob: true,
    })?
    .into_response())
}

#[derive(Deserialize)]
struct TodayQuery {
    member: i64,
}

async fn today_list(
    State(state): State<AppState>,
    Query(query): Query<TodayQuery>,
) -> Result<Response, AppError> {
    let Some(list) = board::today_list(&state.store, query.member, state.today()).await? else {
        return Ok((StatusCode::NOT_FOUND, "That person isn't on the board").into_response());
    };
    Ok(render(TodayPartial {
        today: today_list_view(&list),
        oob: false,
    })?
    .into_response())
}

#[derive(Deserialize)]
struct UndoForm {
    entry: i64,
    item: Option<i64>,
}

async fn undo(
    State(state): State<AppState>,
    Form(form): Form<UndoForm>,
) -> Result<Html<String>, AppError> {
    let now = state.clock.now();
    let today = local_day(now, &state.time_zone);
    let (toast, member, item_id) =
        match rules::undo_entry(&state.store, now, &state.time_zone, form.entry).await {
            Ok(undone) => (
                ToastView {
                    kind: "undone",
                    refusal: "",
                    text: format!("{} {} undone", undone.face, undone.picture),
                    undo_entry: None,
                },
                Some(undone.member_id),
                undone.item_id.or(form.item),
            ),
            Err(CommandError::Refused(refusal)) => (ToastView::refused(refusal), None, form.item),
            Err(CommandError::Store(e)) => return Err(AppError(e)),
        };
    let board = board::load(&state.store, today).await?;
    let item = item_id.and_then(|id| {
        [
            Section::DailyChores,
            Section::GoodDeeds,
            Section::OccasionalChores,
        ]
        .into_iter()
        .find_map(|section| {
            section_items(&board, section)
                .iter()
                .find(|item| item.id == id)
                .map(|item| ItemView {
                    oob: true,
                    ..item_view(&board, section, item)
                })
        })
    });
    let today_list = match member {
        Some(member) => board::today_list(&state.store, member, today).await?,
        None => None,
    };
    render(UndoResponse {
        item,
        members: member_views(&board),
        today: today_list.as_ref().map(today_list_view),
        toast,
        oob: true,
    })
}

/// The live refetch: pots plus every section's rows, all out of band. Other jobs keeps the
/// device's current search, so a refetch never resets what someone is typing.
#[derive(Template)]
#[template(path = "board_refresh.html")]
struct BoardRefresh {
    members: Vec<MemberView>,
    sections: Vec<SectionView>,
    /// The device's open Spend sheet, redrawn so its pot and locks stay current.
    spend: Option<SpendView>,
    oob: bool,
}

/// Sheets open on the refreshing device (sent by `hx-include`).
#[derive(Deserialize)]
struct OpenSheets {
    spend: Option<i64>,
}

async fn board_refresh(
    State(state): State<AppState>,
    Query(search): Query<SearchQuery>,
    Query(open): Query<OpenSheets>,
) -> Result<Html<String>, AppError> {
    let board = board::load(&state.store, state.today()).await?;
    let sections = [
        Section::DailyChores,
        Section::GoodDeeds,
        Section::OccasionalChores,
    ]
    .into_iter()
    .map(|section| SectionView {
        key: section.key(),
        title: section.title(),
        searchable: section == Section::OccasionalChores,
        items: section_items(&board, section)
            .iter()
            .filter(|item| section != Section::OccasionalChores || matches_search(item, &search.q))
            .map(|item| item_view(&board, section, item))
            .collect(),
    })
    .collect();
    let spend = match open.spend {
        Some(member) => spend_view(&board, &board::rewards(&state.store).await?, Some(member)),
        None => None,
    };
    render(BoardRefresh {
        members: member_views(&board),
        sections,
        spend: spend.map(|spend| SpendView { oob: true, ..spend }),
        oob: true,
    })
}

struct RewardView {
    id: i64,
    name: String,
    picture: String,
    price: i64,
    /// Pebbles still needed, if the reward costs more than can be spent.
    more_to_go: Option<i64>,
    /// ⭐ Save for this is offered: priced above the pot, and no goal yet.
    can_save: bool,
}

/// The Spend sheet: "whose pebbles?", then that person's reward tiles.
struct SpendView {
    /// Everyone on the board, to pick whose pebbles to spend.
    picker: Vec<MemberView>,
    /// Whose pebbles are being spent; no tiles until someone is picked.
    member: Option<MemberView>,
    /// What can be spent: the pot, or what's saved (pot + goal price) during a goal.
    spendable: i64,
    rewards: Vec<RewardView>,
    oob: bool,
}

/// The Spend sheet for `member` (or just the picker), or `None` if they aren't on the board.
fn spend_view(board: &Board, rewards: &[Item], member: Option<i64>) -> Option<SpendView> {
    let member = match member {
        Some(id) => Some(board.members.iter().find(|m| m.id == id)?),
        None => None,
    };
    let member = member.map(|m| board_member_view(board, m));
    let spendable = member
        .as_ref()
        .map_or(0, |m| m.goal.as_ref().map_or(m.pot, |g| g.saved));
    let saving = member.as_ref().is_some_and(|m| m.goal.is_some());
    Some(SpendView {
        picker: member_views(board),
        spendable,
        rewards: match member {
            Some(_) => rewards
                .iter()
                .map(|r| RewardView {
                    id: r.id,
                    name: r.name.clone(),
                    picture: r.picture.clone(),
                    price: r.value,
                    more_to_go: (r.value > spendable).then(|| r.value - spendable),
                    can_save: !saving && r.value > spendable,
                })
                .collect(),
            None => Vec::new(),
        },
        member,
        oob: false,
    })
}

#[derive(Template)]
#[template(path = "spend.html")]
struct SpendPartial {
    spend: SpendView,
}

#[derive(Deserialize)]
struct SpendQuery {
    member: Option<i64>,
}

async fn spend_sheet(
    State(state): State<AppState>,
    Query(query): Query<SpendQuery>,
) -> Result<Response, AppError> {
    let board = board::load(&state.store, state.today()).await?;
    let rewards = board::rewards(&state.store).await?;
    let Some(spend) = spend_view(&board, &rewards, query.member) else {
        return Ok((StatusCode::NOT_FOUND, "That person isn't on the board").into_response());
    };
    Ok(render(SpendPartial { spend })?.into_response())
}

/// The reply to a reward tap: everything out of band (pots, the Spend sheet and a toast).
#[derive(Template)]
#[template(path = "redeem_response.html")]
struct RedeemResponse {
    members: Vec<MemberView>,
    spend: Option<SpendView>,
    toast: ToastView,
    oob: bool,
}

async fn redeem(
    State(state): State<AppState>,
    Form(form): Form<EarnForm>,
) -> Result<Html<String>, AppError> {
    let now = state.clock.now();
    let toast =
        match rules::redeem(&state.store, now, &state.time_zone, form.member, form.item).await {
            Ok(redeemed) => ToastView {
                kind: "redeemed",
                refusal: "",
                text: format!("{} {} {}", redeemed.face, redeemed.picture, redeemed.amount),
                undo_entry: Some(redeemed.entry_id),
            },
            Err(CommandError::Refused(refusal)) => ToastView::refused(refusal),
            Err(CommandError::Store(e)) => return Err(AppError(e)),
        };
    let board = board::load(&state.store, local_day(now, &state.time_zone)).await?;
    let rewards = board::rewards(&state.store).await?;
    let spend = spend_view(&board, &rewards, Some(form.member));
    render(RedeemResponse {
        members: member_views(&board),
        spend: spend.map(|spend| SpendView { oob: true, ..spend }),
        toast,
        oob: true,
    })
}

async fn earn_good_deed(
    State(state): State<AppState>,
    Form(form): Form<EarnForm>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let result =
        rules::earn_good_deed(&state.store, now, &state.time_zone, form.member, form.item).await;
    let toast = match result {
        Ok(earned) => ToastView {
            kind: "earned",
            refusal: "",
            text: format!("{} {} +{}", earned.face, earned.picture, earned.amount),
            undo_entry: Some(earned.entry_id),
        },
        Err(CommandError::Refused(refusal)) => ToastView::refused(refusal),
        Err(CommandError::Store(e)) => return Err(AppError(e)),
    };
    let board = board::load(&state.store, local_day(now, &state.time_zone)).await?;
    let Some(item) = board
        .good_deeds
        .iter()
        .find(|item| item.id == form.item)
        .map(|item| item_view(&board, Section::GoodDeeds, item))
    else {
        return Ok((StatusCode::NOT_FOUND, toast.text).into_response());
    };
    Ok(render(EarnResponse {
        item,
        members: member_views(&board),
        toast,
        oob: true,
    })?
    .into_response())
}

#[derive(Deserialize)]
struct MemberForm {
    member: i64,
}

/// The reply to a goal command: like a redeem, everything out of band (pots, the Spend sheet and a
/// toast), since Save for this and Give up come from the Spend sheet and Got it from a pot card.
async fn goal_reply(
    state: &AppState,
    now: jiff::Timestamp,
    member: i64,
    toast: ToastView,
) -> Result<Html<String>, AppError> {
    let board = board::load(&state.store, local_day(now, &state.time_zone)).await?;
    let rewards = board::rewards(&state.store).await?;
    let spend = spend_view(&board, &rewards, Some(member));
    render(RedeemResponse {
        members: member_views(&board),
        spend: spend.map(|spend| SpendView { oob: true, ..spend }),
        toast,
        oob: true,
    })
}

fn goal_toast(
    result: Result<rules::GoalChanged, CommandError>,
    kind: &'static str,
    text: impl FnOnce(&rules::GoalChanged) -> String,
) -> Result<ToastView, AppError> {
    match result {
        Ok(changed) => Ok(ToastView {
            kind,
            refusal: "",
            text: text(&changed),
            undo_entry: changed.entry_id,
        }),
        Err(CommandError::Refused(refusal)) => Ok(ToastView::refused(refusal)),
        Err(CommandError::Store(e)) => Err(AppError(e)),
    }
}

async fn set_goal(
    State(state): State<AppState>,
    Form(form): Form<EarnForm>,
) -> Result<Html<String>, AppError> {
    let now = state.clock.now();
    let result = rules::set_goal(&state.store, now, &state.time_zone, form.member, form.item).await;
    let toast = goal_toast(result, "goal_set", |g| {
        format!("{} {} {} ⭐ Saving!", g.face, g.picture, g.amount)
    })?;
    goal_reply(&state, now, form.member, toast).await
}

async fn give_up_goal(
    State(state): State<AppState>,
    Form(form): Form<MemberForm>,
) -> Result<Html<String>, AppError> {
    let now = state.clock.now();
    let result = rules::give_up_goal(&state.store, now, &state.time_zone, form.member).await;
    let toast = goal_toast(result, "goal_given_up", |g| {
        format!("{} {} +{}", g.face, g.picture, g.amount)
    })?;
    goal_reply(&state, now, form.member, toast).await
}

async fn goal_got_it(
    State(state): State<AppState>,
    Form(form): Form<MemberForm>,
) -> Result<Html<String>, AppError> {
    let now = state.clock.now();
    let result = rules::hand_over_goal(&state.store, form.member).await;
    let toast = goal_toast(result, "handed_over", |g| {
        format!("{} {} 🎉", g.face, g.picture)
    })?;
    goal_reply(&state, now, form.member, toast).await
}
