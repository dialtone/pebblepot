//! ✏️ Edit mode for chores and good deeds: change, reorder and retire rows. Edits never touch
//! the log or the pots.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{BoardPage, Fragment, HOST, TestApp};
use jiff::SignedDuration;
use pebblepot::web::{ITEM_EDIT_PATH, ITEM_REORDER_PATH, ITEM_RETIRE_PATH};
use scraper::{ElementRef, Html, Selector};
use tower::ServiceExt;

const SECTIONS: [&str; 3] = ["daily_chores", "good_deeds", "occasional_chores"];

fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    let sel = Selector::parse(selector).unwrap();
    html.select(&sel).collect()
}

/// Percent-encodes a form value (everything but ASCII letters and digits).
fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// The board in edit mode, reached through the ✏️ Edit toggle.
async fn edit_board(app: &TestApp) -> BoardPage {
    let board = app.board().await;
    let toggle = select(&board.0, "[data-edit-toggle]");
    assert_eq!(toggle.len(), 1, "no ✏️ Edit toggle on the board");
    let href = toggle[0].value().attr("href").unwrap().to_string();
    let (status, body) = app.get(&href).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    BoardPage(Html::parse_document(&body))
}

/// What the pencil's sheet is filled in with. `section: None` sends no section (a good deed's
/// sheet has no daily/occasional choice).
struct Change<'a> {
    name: &'a str,
    picture: &'a str,
    value: &'a str,
    section: Option<&'a str>,
}

fn change_form(item: &str, change: &Change) -> String {
    let mut form = format!(
        "item={item}&name={}&picture={}&value={}",
        enc(change.name),
        enc(change.picture),
        enc(change.value)
    );
    if let Some(section) = change.section {
        form.push_str(&format!("&section={section}"));
    }
    form
}

/// Opens an item's ✏️ pencil sheet from the edit-mode board.
async fn open_pencil(app: &TestApp, item: &str) -> Html {
    let board = edit_board(app).await;
    let sel = format!("[data-item-name=\"{item}\"] [data-edit-item]");
    let pencil = select(&board.0, &sel);
    assert_eq!(pencil.len(), 1, "no pencil on {item}");
    let path = pencil[0].value().attr("hx-get").unwrap().to_string();
    let (status, body) = app.get(&path).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Html::parse_fragment(&body)
}

/// Saves the pencil sheet for `item` (by name). Returns the reply.
async fn change(app: &TestApp, item: &str, change: Change<'_>) -> Fragment {
    let id = app.board().await.item_id(item);
    let (status, body) = app
        .post_form(ITEM_EDIT_PATH, &change_form(&id, &change))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fragment(Html::parse_fragment(&body))
}

/// Taps 🗑 Retire on `item` (by id). Returns the reply.
async fn retire_id(app: &TestApp, id: &str) -> Fragment {
    let (status, body) = app.post_form(ITEM_RETIRE_PATH, &format!("item={id}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fragment(Html::parse_fragment(&body))
}

async fn retire(app: &TestApp, item: &str) -> Fragment {
    let id = app.board().await.item_id(item);
    retire_id(app, &id).await
}

/// Drops a dragged row: posts the section's new order (item names).
async fn reorder(app: &TestApp, section: &str, names: &[&str]) -> Fragment {
    let board = app.board().await;
    let ids: Vec<String> = names.iter().map(|n| board.item_id(n)).collect();
    let (status, body) = app
        .post_form(
            ITEM_REORDER_PATH,
            &format!("section={section}&order={}", ids.join("%2C")),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fragment(Html::parse_fragment(&body))
}

/// The rows of `section` swapped in out of band by a reply, by name.
fn oob_rows(reply: &Fragment, section: &str) -> Option<Vec<String>> {
    let container = select(&reply.0, &format!("#rows-{section}[hx-swap-oob]"))
        .into_iter()
        .next()?;
    let rows = Selector::parse("[data-item]").unwrap();
    Some(
        container
            .select(&rows)
            .map(|r| r.value().attr("data-item-name").unwrap().to_string())
            .collect(),
    )
}

fn closes_sheet(reply: &Fragment) -> bool {
    select(&reply.0, "#sheet[hx-swap-oob]")
        .first()
        .is_some_and(|s| s.inner_html().trim().is_empty())
}

/// A row's picture and pebble value, as the board shows them.
fn row(board: &BoardPage, item: &str) -> (String, i64) {
    let rows = select(&board.0, &format!("[data-item-name=\"{item}\"]"));
    assert_eq!(rows.len(), 1, "{item} isn't on the board once");
    let picture = rows[0]
        .select(&Selector::parse(".picture").unwrap())
        .next()
        .unwrap()
        .text()
        .collect::<String>();
    let value = rows[0].value().attr("data-value").unwrap().parse().unwrap();
    (picture.trim().to_string(), value)
}

fn all_items(board: &BoardPage) -> Vec<String> {
    SECTIONS.iter().flat_map(|s| board.items(s)).collect()
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

// ---------------------------------------------------------------------------------------------
// The toggle

#[tokio::test]
async fn the_board_has_an_edit_toggle_and_no_edit_controls() {
    let app = TestApp::start().await;
    let board = app.board().await;
    assert_eq!(select(&board.0, "[data-edit-toggle]").len(), 1);
    assert!(select(&board.0, "[data-edit-mode]").is_empty());
    for marker in ["[data-edit-item]", "[data-drag-handle]", "[data-retire]"] {
        assert!(
            select(&board.0, marker).is_empty(),
            "{marker} outside edit mode"
        );
    }
}

#[tokio::test]
async fn edit_mode_gives_every_row_a_pencil_a_drag_handle_and_retire_and_no_faces() {
    let app = TestApp::start().await;
    let normal = app.board().await;
    let edit = edit_board(&app).await;
    assert_eq!(select(&edit.0, "[data-edit-mode]").len(), 1);
    for section in SECTIONS {
        // Same rows, same order.
        assert_eq!(edit.items(section), normal.items(section), "{section}");
        let rows = select(&edit.0, &format!("[data-section={section}] [data-item]"));
        for r in rows {
            let id = r.value().attr("data-item").unwrap();
            let has = |marker: &str| {
                r.select(&Selector::parse(marker).unwrap())
                    .next()
                    .map(|e| e.value().clone())
            };
            let pencil = has("[data-edit-item]").expect("no pencil");
            assert_eq!(pencil.attr("data-edit-item"), Some(id));
            let handle = has("[data-drag-handle]").expect("no drag handle");
            assert_eq!(handle.attr("data-reorder-section"), Some(section));
            let retire = has("[data-retire]").expect("no 🗑 Retire");
            assert_eq!(retire.attr("hx-post"), Some(ITEM_RETIRE_PATH));
            assert!(has("[data-face-for]").is_none(), "faces in edit mode");
        }
        // Something else earns pebbles, so it waits until edit mode is left.
        assert!(
            select(
                &edit.0,
                &format!("[data-section={section}] [data-something-else]")
            )
            .is_empty()
        );
    }
    // The pots are still there; no PIN or sign-in on the way in.
    assert_eq!(edit.pots(), normal.pots());
}

#[tokio::test]
async fn done_leaves_edit_mode_for_the_normal_board() {
    let app = TestApp::start().await;
    let edit = edit_board(&app).await;
    let done = select(&edit.0, "[data-edit-done]");
    assert_eq!(done.len(), 1);
    let href = done[0].value().attr("href").unwrap().to_string();
    let (status, body) = app.get(&href).await;
    assert_eq!(status, StatusCode::OK);
    let board = BoardPage(Html::parse_document(&body));
    assert!(select(&board.0, "[data-edit-mode]").is_empty());
    assert!(select(&board.0, "[data-edit-item]").is_empty());
    assert!(!select(&board.0, "[data-face-for]").is_empty());
}

#[tokio::test]
async fn the_live_refresh_and_the_search_keep_edit_mode_on_the_device_that_is_in_it() {
    let app = TestApp::start().await;
    let edit = edit_board(&app).await;
    // The live element and the search box send the edit-mode marker along.
    let live = select(&edit.0, "#live");
    assert!(
        live[0]
            .value()
            .attr("hx-include")
            .unwrap()
            .contains("[data-edit-mode]")
    );
    let search = select(&edit.0, "[data-search]");
    assert!(
        search[0]
            .value()
            .attr("hx-include")
            .unwrap()
            .contains("[data-edit-mode]")
    );
    let marker = select(&edit.0, "[data-edit-mode]");
    let (name, value) = (
        marker[0].value().attr("name").unwrap(),
        marker[0].value().attr("value").unwrap(),
    );

    let (_, body) = app.get(&format!("/board?{name}={value}")).await;
    let refresh = Html::parse_fragment(&body);
    assert!(!select(&refresh, "[data-edit-item]").is_empty());
    assert!(select(&refresh, "[data-face-for]").is_empty());

    let (_, body) = app
        .get(&format!("/occasional?q=leaves&{name}={value}"))
        .await;
    let found = Html::parse_fragment(&body);
    assert_eq!(select(&found, "[data-edit-item]").len(), 1);
    assert!(select(&found, "[data-face-for]").is_empty());

    // Other devices, not in edit mode, still get faces.
    let (_, body) = app.get("/board").await;
    let refresh = Html::parse_fragment(&body);
    assert!(select(&refresh, "[data-edit-item]").is_empty());
    assert!(!select(&refresh, "[data-face-for]").is_empty());
}

// ---------------------------------------------------------------------------------------------
// The pencil

#[tokio::test]
async fn the_pencil_sheet_starts_with_the_item_as_it_is() {
    let app = TestApp::start().await;
    let sheet = open_pencil(&app, "Tidy toys").await;
    let id = app.board().await.item_id("Tidy toys");
    assert_eq!(
        select(&sheet, "[data-edit-sheet]")[0]
            .value()
            .attr("data-edit-sheet"),
        Some(id.as_str())
    );
    let field = |name: &str| {
        select(&sheet, &format!("form input[name={name}]"))[0]
            .value()
            .attr("value")
            .map(str::to_string)
    };
    assert_eq!(field("item").as_deref(), Some(id.as_str()));
    assert_eq!(field("name").as_deref(), Some("Tidy toys"));
    assert_eq!(field("picture").as_deref(), Some("🧸"));
    assert_eq!(field("value").as_deref(), Some("2"));
    // A chore can be daily or occasional; it starts as what it is.
    let choices: Vec<(String, bool)> = select(&sheet, "form input[type=radio][name=section]")
        .iter()
        .map(|c| {
            (
                c.value().attr("value").unwrap().to_string(),
                c.value().attr("checked").is_some(),
            )
        })
        .collect();
    assert_eq!(
        choices,
        [
            ("daily_chores".to_string(), true),
            ("occasional_chores".to_string(), false)
        ]
    );

    let sheet = open_pencil(&app, "Rake leaves").await;
    let checked = select(&sheet, "form input[name=section][checked]");
    assert_eq!(checked[0].value().attr("value"), Some("occasional_chores"));
}

#[tokio::test]
async fn a_good_deed_has_no_daily_or_occasional_choice() {
    let app = TestApp::start().await;
    let sheet = open_pencil(&app, "Shared nicely").await;
    assert!(select(&sheet, "form input[type=radio][name=section]").is_empty());
    assert_eq!(select(&sheet, "form input[name=value]").len(), 1);
}

#[tokio::test]
async fn changing_a_chores_name_picture_and_value_shows_on_the_board() {
    let app = TestApp::start().await;
    let before = app.board().await.items("daily_chores");
    let reply = change(
        &app,
        "Make bed",
        Change {
            name: "Make the bed",
            picture: "🛌",
            value: "3",
            section: Some("daily_chores"),
        },
    )
    .await;
    assert_eq!(reply.toast().kind, "edited");
    assert!(closes_sheet(&reply));
    // Same place in the section, new name; redrawn in edit mode on this device.
    let mut expected = before.clone();
    expected[0] = "Make the bed".into();
    assert_eq!(oob_rows(&reply, "daily_chores"), Some(expected.clone()));
    assert!(!select(&reply.0, "#rows-daily_chores [data-edit-item]").is_empty());

    let board = app.board().await;
    assert_eq!(board.items("daily_chores"), expected);
    assert_eq!(row(&board, "Make the bed"), ("🛌".to_string(), 3));
    // Earning it now uses the new name, picture and value.
    let earned = app.earn_chore("Kid 1", "Make the bed").await;
    assert_eq!(earned.toast().text, "🦊 🛌 +3");
    assert_eq!(app.pot("Kid 1").await, 3);
    assert_eq!(app.today_list("Kid 1").await.names(), ["Make the bed"]);
}

#[tokio::test]
async fn changing_a_good_deed_shows_on_the_board_and_earns_the_new_value() {
    let app = TestApp::start().await;
    change(
        &app,
        "Kind words",
        Change {
            name: "Kind words to a friend",
            picture: "💬",
            value: "4",
            section: None,
        },
    )
    .await;
    let board = app.board().await;
    assert!(
        board
            .items("good_deeds")
            .contains(&"Kind words to a friend".into())
    );
    assert!(!all_items(&board).contains(&"Kind words".into()));
    assert_eq!(row(&board, "Kind words to a friend"), ("💬".to_string(), 4));

    let kid = board.member_id("Kid 2");
    let item = board.item_id("Kind words to a friend");
    let (_, body) = app
        .post_form(
            pebblepot::web::EARN_GOOD_DEED_PATH,
            &format!("member={kid}&item={item}"),
        )
        .await;
    assert_eq!(
        Fragment(Html::parse_fragment(&body)).toast().text,
        "🐻 💬 +4"
    );
    assert_eq!(app.pot("Kid 2").await, 4);
}

#[tokio::test]
async fn a_daily_chore_switched_to_occasional_moves_to_the_end_of_other_jobs() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Tidy toys").await;
    let others = app.board().await.items("occasional_chores");
    let reply = change(
        &app,
        "Tidy toys",
        Change {
            name: "Tidy toys",
            picture: "🧸",
            value: "2",
            section: Some("occasional_chores"),
        },
    )
    .await;
    let mut expected = others.clone();
    expected.push("Tidy toys".into());
    assert_eq!(
        oob_rows(&reply, "occasional_chores"),
        Some(expected.clone())
    );
    assert!(
        !oob_rows(&reply, "daily_chores")
            .unwrap()
            .contains(&"Tidy toys".into())
    );

    let board = app.board().await;
    assert!(!board.items("daily_chores").contains(&"Tidy toys".into()));
    assert_eq!(board.items("occasional_chores"), expected);
    // Still done today by Kid 1, and still once a day.
    assert_eq!(board.done_by("Tidy toys"), ["Kid 1"]);
    let again = app.earn_chore("Kid 1", "Tidy toys").await;
    assert_eq!(again.toast().refusal.as_deref(), Some("already_done_today"));
    assert_eq!(app.pot("Kid 1").await, 2);
    // It turns up in the Other jobs search.
    let (_, body) = app.get("/occasional?q=toys").await;
    assert_eq!(
        Fragment(Html::parse_fragment(&body)).item_names(),
        ["Tidy toys"]
    );
}

#[tokio::test]
async fn an_occasional_chore_switched_to_daily_moves_to_the_end_of_todays_jobs() {
    let app = TestApp::start().await;
    let daily = app.board().await.items("daily_chores");
    change(
        &app,
        "Rake leaves",
        Change {
            name: "Rake leaves",
            picture: "🍂",
            value: "3",
            section: Some("daily_chores"),
        },
    )
    .await;
    let board = app.board().await;
    let mut expected = daily.clone();
    expected.push("Rake leaves".into());
    assert_eq!(board.items("daily_chores"), expected);
    assert!(
        !board
            .items("occasional_chores")
            .contains(&"Rake leaves".into())
    );
}

#[tokio::test]
async fn a_good_deed_cant_become_a_chore() {
    let app = TestApp::start().await;
    let reply = change(
        &app,
        "Shared nicely",
        Change {
            name: "Shared nicely",
            picture: "🤝",
            value: "2",
            section: Some("daily_chores"),
        },
    )
    .await;
    assert_eq!(reply.toast().kind, "refused");
    let board = app.board().await;
    assert!(board.items("good_deeds").contains(&"Shared nicely".into()));
    assert!(
        !board
            .items("daily_chores")
            .contains(&"Shared nicely".into())
    );
}

#[tokio::test]
async fn a_blank_name_or_a_value_out_of_range_is_refused_and_the_sheet_stays_open() {
    let app = TestApp::start().await;
    for (name, value, refusal) in [
        ("   ", "2", "name_needed"),
        ("Make bed", "0", "bad_value"),
        ("Make bed", "101", "bad_value"),
        ("Make bed", "-1", "bad_value"),
        ("Make bed", "", "bad_value"),
        ("Make bed", "lots", "bad_value"),
    ] {
        let reply = change(
            &app,
            "Make bed",
            Change {
                name,
                picture: "🛌",
                value,
                section: Some("daily_chores"),
            },
        )
        .await;
        assert_eq!(
            reply.toast().refusal.as_deref(),
            Some(refusal),
            "{name:?} {value:?}"
        );
        assert!(!closes_sheet(&reply), "{name:?} {value:?}");
        assert!(oob_rows(&reply, "daily_chores").is_none());
    }
    // Unchanged.
    assert_eq!(row(&app.board().await, "Make bed"), ("🛏️".to_string(), 1));
}

#[tokio::test]
async fn the_value_can_go_to_1_and_to_100() {
    let app = TestApp::start().await;
    for value in ["1", "100"] {
        let reply = change(
            &app,
            "Tidy toys",
            Change {
                name: "Tidy toys",
                picture: "🧸",
                value,
                section: Some("daily_chores"),
            },
        )
        .await;
        assert_eq!(reply.toast().kind, "edited", "{value}");
        assert_eq!(
            row(&app.board().await, "Tidy toys").1,
            value.parse::<i64>().unwrap()
        );
    }
}

#[tokio::test]
async fn a_blank_picture_becomes_sparkles() {
    let app = TestApp::start().await;
    change(
        &app,
        "Make bed",
        Change {
            name: "Make bed",
            picture: " ",
            value: "1",
            section: Some("daily_chores"),
        },
    )
    .await;
    assert_eq!(row(&app.board().await, "Make bed").0, "✨");
}

// ---------------------------------------------------------------------------------------------
// Edits never touch the log or the pots

#[tokio::test]
async fn editing_a_value_changes_no_pot_and_no_entry_already_in_the_log() {
    // Kept in its section, and switched from daily to occasional.
    for section in ["daily_chores", "occasional_chores"] {
        let app = TestApp::start().await;
        // Yesterday and today.
        app.earn_chore("Kid 1", "Make bed").await;
        app.clock.advance(SignedDuration::from_hours(24));
        app.earn_chore("Kid 1", "Make bed").await;
        app.earn_chore("Mom", "Make bed").await;
        assert_eq!(app.pot("Kid 1").await, 2);

        change(
            &app,
            "Make bed",
            Change {
                name: "Make the bed",
                picture: "🛌",
                value: "5",
                section: Some(section),
            },
        )
        .await;

        assert_eq!(app.pot("Kid 1").await, 2);
        assert_eq!(app.pot("Mom").await, 1);
        let today = app.today_list("Kid 1").await.entries();
        assert_eq!(today.len(), 1);
        assert_eq!((today[0].name.as_str(), today[0].amount), ("Make bed", 1));
        let list = app.today_list("Kid 1").await;
        let picture = select(&list.0, "[data-entry] .picture")[0]
            .text()
            .collect::<String>();
        assert_eq!(picture, "🛏️");
        // Undoing that entry takes off what it earned (1), not the new value.
        let reply = app.undo(&today[0].entry).await;
        assert_eq!(reply.toast().kind, "undone");
        assert_eq!(app.pot("Kid 1").await, 1);
        // A new earn uses the new value, next to the old copy.
        app.earn_chore("Kid 2", "Make the bed").await;
        assert_eq!(app.pot("Kid 2").await, 5);
        assert_eq!(app.pot("Mom").await, 1);
        assert_eq!(app.today_list("Mom").await.names(), ["Make bed"]);
    }
}

// ---------------------------------------------------------------------------------------------
// 🗑 Retire

#[tokio::test]
async fn retiring_a_chore_takes_it_off_the_board_everywhere() {
    let app = TestApp::start().await;
    let before = app.board().await.items("daily_chores");
    let reply = retire(&app, "Make bed").await;
    assert_eq!(reply.toast().kind, "retired");
    let expected: Vec<String> = before[1..].to_vec();
    assert_eq!(oob_rows(&reply, "daily_chores"), Some(expected.clone()));

    let board = app.board().await;
    assert_eq!(board.items("daily_chores"), expected);
    assert!(!all_items(&edit_board(&app).await).contains(&"Make bed".into()));
    let (_, body) = app.get("/board").await;
    assert!(
        !Fragment(Html::parse_fragment(&body))
            .item_names()
            .contains(&"Make bed".into())
    );

    // An occasional chore leaves the search too.
    retire(&app, "Rake leaves").await;
    let (_, body) = app.get("/occasional?q=leaves").await;
    assert!(
        Fragment(Html::parse_fragment(&body))
            .item_names()
            .is_empty()
    );
}

#[tokio::test]
async fn a_retired_items_log_entries_keep_their_name_picture_and_amount() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Tidy toys").await;
    let board = app.board().await;
    let (kid, deed) = (board.member_id("Kid 2"), board.item_id("Was brave"));
    app.post_form(
        pebblepot::web::EARN_GOOD_DEED_PATH,
        &format!("member={kid}&item={deed}"),
    )
    .await;
    assert_eq!(app.pot("Kid 2").await, 4);

    retire(&app, "Tidy toys").await;
    retire(&app, "Was brave").await;

    assert_eq!(app.pot("Kid 2").await, 4);
    let list = app.today_list("Kid 2").await;
    let entries = list.entries();
    assert_eq!(
        entries
            .iter()
            .map(|e| (e.name.as_str(), e.amount))
            .collect::<Vec<_>>(),
        [("Was brave", 2), ("Tidy toys", 2)]
    );
    let pictures: Vec<String> = select(&list.0, "[data-entry] .picture")
        .iter()
        .map(|p| p.text().collect())
        .collect();
    assert_eq!(pictures, ["🦁", "🧸"]);

    // They can still be undone today, taking off what they earned.
    let reply = app.undo(&entries[1].entry).await;
    assert_eq!(reply.toast().kind, "undone");
    assert_eq!(app.pot("Kid 2").await, 2);
}

#[tokio::test]
async fn a_retired_item_cant_be_earned_edited_or_retired_again() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let (kid, chore, deed) = (
        board.member_id("Kid 1"),
        board.item_id("Make bed"),
        board.item_id("Said sorry"),
    );
    retire_id(&app, &chore).await;
    retire_id(&app, &deed).await;

    let (status, _) = app
        .post_form(
            pebblepot::web::EARN_CHORE_PATH,
            &format!("member={kid}&item={chore}"),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .post_form(
            pebblepot::web::EARN_GOOD_DEED_PATH,
            &format!("member={kid}&item={deed}"),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.pot("Kid 1").await, 0);

    let (status, _) = app
        .post_form(ITEM_RETIRE_PATH, &format!("item={chore}"))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .post_form(
            ITEM_EDIT_PATH,
            &change_form(
                &chore,
                &Change {
                    name: "Back again",
                    picture: "🛏️",
                    value: "1",
                    section: Some("daily_chores"),
                },
            ),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app.get(&format!("{ITEM_EDIT_PATH}?item={chore}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!all_items(&app.board().await).contains(&"Back again".into()));
}

#[tokio::test]
async fn a_kept_something_else_can_be_retired_after_its_undo() {
    let app = TestApp::start().await;
    let kid = app.board().await.member_id("Kid 1");
    let (_, body) = app
        .post_form(
            pebblepot::web::SOMETHING_ELSE_PATH,
            &format!("section=daily_chores&member={kid}&name=Feed%20the%20fish&picture=%F0%9F%90%9F&value=2&keep=on"),
        )
        .await;
    let entry = Fragment(Html::parse_fragment(&body))
        .toast_undo_entry()
        .unwrap();
    app.undo(&entry).await;
    assert!(
        app.board()
            .await
            .items("daily_chores")
            .contains(&"Feed the fish".into())
    );

    retire(&app, "Feed the fish").await;
    assert!(
        !app.board()
            .await
            .items("daily_chores")
            .contains(&"Feed the fish".into())
    );
    assert_eq!(app.pot("Kid 1").await, 0);
}

// ---------------------------------------------------------------------------------------------
// Drag handles

#[tokio::test]
async fn rows_can_be_reordered_within_a_section_and_the_order_persists() {
    let mut app = TestApp::start().await;
    let before = app.board().await.items("good_deeds");
    let mut order: Vec<String> = before.clone();
    // Drag the last good deed to the top.
    let last = order.pop().unwrap();
    order.insert(0, last);
    let names: Vec<&str> = order.iter().map(String::as_str).collect();
    let reply = reorder(&app, "good_deeds", &names).await;
    assert_eq!(oob_rows(&reply, "good_deeds"), Some(order.clone()));

    assert_eq!(app.board().await.items("good_deeds"), order);
    assert_eq!(edit_board(&app).await.items("good_deeds"), order);
    app.restart().await;
    assert_eq!(app.board().await.items("good_deeds"), order);
    // Other sections are untouched.
    assert_eq!(
        app.board().await.items("daily_chores")[..2],
        s(&["Make bed", "Get dressed"])
    );
}

#[tokio::test]
async fn reordering_a_searched_list_moves_only_the_rows_that_were_shown() {
    let app = TestApp::start().await;
    let before = app.board().await.items("occasional_chores");
    // Searching "the" shows a few rows; swap the first and last shown.
    let (_, body) = app.get("/occasional?q=the").await;
    let shown = Fragment(Html::parse_fragment(&body)).item_names();
    assert!(shown.len() >= 2, "{shown:?}");
    let mut dropped = shown.clone();
    let n = dropped.len();
    dropped.swap(0, n - 1);
    let names: Vec<&str> = dropped.iter().map(String::as_str).collect();
    reorder(&app, "occasional_chores", &names).await;

    let after = app.board().await.items("occasional_chores");
    let expected: Vec<String> = before
        .iter()
        .map(|name| {
            if *name == shown[0] {
                shown[n - 1].clone()
            } else if *name == shown[n - 1] {
                shown[0].clone()
            } else {
                name.clone()
            }
        })
        .collect();
    assert_eq!(after, expected);
}

#[tokio::test]
async fn a_new_something_else_goes_after_reordered_rows() {
    let app = TestApp::start().await;
    let before = app.board().await.items("daily_chores");
    let mut order = before.clone();
    order.reverse();
    let names: Vec<&str> = order.iter().map(String::as_str).collect();
    reorder(&app, "daily_chores", &names).await;
    let kid = app.board().await.member_id("Kid 1");
    app.post_form(
        pebblepot::web::SOMETHING_ELSE_PATH,
        &format!("section=daily_chores&member={kid}&name=Water%20plants&value=1&keep=on"),
    )
    .await;
    let mut expected = order.clone();
    expected.push("Water plants".into());
    assert_eq!(app.board().await.items("daily_chores"), expected);
}

#[tokio::test]
async fn a_reorder_with_rows_from_another_section_retired_or_repeated_is_refused() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let daily = board.items("daily_chores");
    let first = board.item_id(&daily[0]);
    let second = board.item_id(&daily[1]);
    let deed = board.item_id("Shared nicely");
    let retired = board.item_id(&daily[2]);
    retire_id(&app, &retired).await;
    let daily = app.board().await.items("daily_chores");

    for order in [
        format!("{second},{deed},{first}"),
        format!("{second},{retired},{first}"),
        format!("{second},{first},{second}"),
        format!("{second},nonsense"),
        "999999".to_string(),
    ] {
        let (status, body) = app
            .post_form(
                ITEM_REORDER_PATH,
                &format!("section=daily_chores&order={}", enc(&order)),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let reply = Fragment(Html::parse_fragment(&body));
        assert_eq!(
            reply.toast().refusal.as_deref(),
            Some("unknown_item"),
            "{order}"
        );
        assert_eq!(app.board().await.items("daily_chores"), daily, "{order}");
    }
    let (status, _) = app
        .post_form(ITEM_REORDER_PATH, &format!("section=rewards&order={first}"))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------------------------
// Edges

#[tokio::test]
async fn rewards_and_unknown_items_arent_edited_from_the_board() {
    let app = TestApp::start().await;
    // A reward's id: from the Spend sheet.
    let kid = app.board().await.member_id("Kid 1");
    let (_, body) = app.get(&format!("/spend?member={kid}")).await;
    let spend = Html::parse_fragment(&body);
    let reward = select(&spend, "[data-reward]")[0]
        .value()
        .attr("data-reward")
        .unwrap()
        .to_string();
    for id in [reward.as_str(), "999999"] {
        let (status, _) = app.get(&format!("{ITEM_EDIT_PATH}?item={id}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
        let (status, _) = app
            .post_form(
                ITEM_EDIT_PATH,
                &change_form(
                    id,
                    &Change {
                        name: "Changed",
                        picture: "🎁",
                        value: "1",
                        section: None,
                    },
                ),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
        let (status, _) = app.post_form(ITEM_RETIRE_PATH, &format!("item={id}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
    }
    let (_, body) = app.get(&format!("/spend?member={kid}")).await;
    let spend = Html::parse_fragment(&body);
    assert!(select(&spend, "[data-reward-name=\"Changed\"]").is_empty());
    assert_eq!(
        select(&spend, &format!("[data-reward=\"{reward}\"]")).len(),
        1
    );
}

#[tokio::test]
async fn edits_reach_every_open_board() {
    let app = TestApp::start().await;
    let mut events = app
        .router
        .clone()
        .oneshot(
            Request::get(pebblepot::web::EVENTS_PATH)
                .header("host", HOST)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .into_body();
    retire(&app, "Make bed").await;
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        use http_body_util::BodyExt;
        loop {
            let frame = events.frame().await.unwrap().unwrap();
            if let Some(data) = frame.data_ref() {
                let text = String::from_utf8_lossy(data).to_string();
                if text.contains("changed") {
                    return text;
                }
            }
        }
    })
    .await
    .expect("no changed event after a retire");
    assert!(frame.contains("data: "), "{frame}");
}

#[tokio::test]
async fn cross_site_edit_posts_are_rejected() {
    let app = TestApp::start().await;
    let id = app.board().await.item_id("Make bed");
    for (path, form) in [
        (
            ITEM_EDIT_PATH,
            change_form(
                &id,
                &Change {
                    name: "Hacked",
                    picture: "💀",
                    value: "100",
                    section: Some("daily_chores"),
                },
            ),
        ),
        (ITEM_RETIRE_PATH, format!("item={id}")),
        (
            ITEM_REORDER_PATH,
            format!("section=daily_chores&order={id}"),
        ),
    ] {
        let (status, _) = app
            .send(
                Request::post(path)
                    .header("host", HOST)
                    .header("origin", "http://evil.example")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
    assert_eq!(row(&app.board().await, "Make bed"), ("🛏️".to_string(), 1));
}
