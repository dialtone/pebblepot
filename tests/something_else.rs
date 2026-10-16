//! ✏️ Something else: add a new chore or good deed and earn it in one step, or record a one-off.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{BoardPage, Fragment, HOST, Pot, TestApp};
use jiff::SignedDuration;
use pebblepot::web::SOMETHING_ELSE_PATH;
use scraper::{Html, Selector};

const SECTIONS: [&str; 3] = ["daily_chores", "good_deeds", "occasional_chores"];

/// What the Something else sheet asks for. `value: None` leaves the pebble value out of the form.
struct Ask<'a> {
    section: &'a str,
    member: &'a str,
    name: &'a str,
    picture: &'a str,
    value: Option<i64>,
    keep: bool,
}

impl<'a> Ask<'a> {
    fn new(section: &'a str, member: &'a str, name: &'a str) -> Self {
        Ask {
            section,
            member,
            name,
            picture: "🦄",
            value: Some(3),
            keep: true,
        }
    }
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

/// The form a Something else sheet posts; `member_id` is a raw id.
fn form(ask: &Ask, member_id: &str) -> String {
    let mut form = format!(
        "section={}&member={member_id}&name={}&picture={}",
        ask.section,
        enc(ask.name),
        enc(ask.picture)
    );
    if let Some(value) = ask.value {
        form.push_str(&format!("&value={value}"));
    }
    if ask.keep {
        form.push_str("&keep=on");
    }
    form
}

/// Fills in and sends the Something else sheet. Returns the response fragment.
async fn something_else(app: &TestApp, ask: Ask<'_>) -> Fragment {
    let member = app.board().await.member_id(ask.member);
    let (status, body) = app
        .post_form(SOMETHING_ELSE_PATH, &form(&ask, &member))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fragment(Html::parse_fragment(&body))
}

/// Opens the Something else sheet from a section's ✏️ button, as the board does.
async fn open_sheet(app: &TestApp, section: &str) -> Html {
    let board = app.board().await;
    let sel = Selector::parse(&format!("[data-section={section}] [data-something-else]")).unwrap();
    let button = board
        .0
        .select(&sel)
        .next()
        .unwrap_or_else(|| panic!("no ✏️ Something else in {section}"));
    let path = button.value().attr("hx-get").unwrap().to_string();
    let (status, body) = app.get(&path).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Html::parse_fragment(&body)
}

fn select<'a>(html: &'a Html, selector: &str) -> Vec<scraper::ElementRef<'a>> {
    let sel = Selector::parse(selector).unwrap();
    html.select(&sel).collect()
}

/// Every item name on the board, in every section.
fn all_items(board: &BoardPage) -> Vec<String> {
    SECTIONS.iter().flat_map(|s| board.items(s)).collect()
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

/// Whether the reply closes the open sheet.
fn closes_sheet(reply: &Fragment) -> bool {
    select(&reply.0, "#sheet[hx-swap-oob]")
        .first()
        .is_some_and(|s| s.inner_html().trim().is_empty())
}

#[tokio::test]
async fn every_section_has_a_something_else_button_that_opens_its_sheet() {
    let app = TestApp::start().await;
    for section in SECTIONS {
        let sheet = open_sheet(&app, section).await;
        let root = select(&sheet, "[data-something-else-sheet]");
        assert_eq!(
            root[0].value().attr("data-something-else-sheet"),
            Some(section)
        );
        let field = select(&sheet, "form [name=section]");
        assert_eq!(field[0].value().attr("value"), Some(section));
        for name in ["name", "picture", "value"] {
            assert_eq!(
                select(&sheet, &format!("form [name={name}]")).len(),
                1,
                "{section}: no {name} field"
            );
        }
        // "Who did it": one choice per family member, in board order, none picked yet.
        let choices: Vec<String> = select(&sheet, "form input[name=member]")
            .iter()
            .map(|c| c.value().attr("value").unwrap().to_string())
            .collect();
        assert_eq!(choices, app.board().await.member_ids(), "{section}");
        assert!(select(&sheet, "form input[name=member][checked]").is_empty());
    }
}

#[tokio::test]
async fn the_pebble_value_starts_at_1_and_keep_on_the_board_starts_on() {
    let app = TestApp::start().await;
    for section in SECTIONS {
        let sheet = open_sheet(&app, section).await;
        let value = select(&sheet, "form input[name=value]");
        assert_eq!(value[0].value().attr("value"), Some("1"), "{section}");
        assert_eq!(value[0].value().attr("min"), Some("1"), "{section}");
        let keep = select(&sheet, "form input[type=checkbox][name=keep]");
        assert_eq!(keep.len(), 1, "{section}");
        assert!(keep[0].value().attr("checked").is_some(), "{section}");
    }
}

#[tokio::test]
async fn kept_on_the_board_it_becomes_a_new_daily_chore_and_is_earned() {
    let app = TestApp::start().await;
    let before = app.board().await.items("daily_chores");
    let reply = something_else(&app, Ask::new("daily_chores", "Kid 1", "Feed the fish")).await;

    assert_eq!(reply.toast().kind, "earned");
    assert_eq!(reply.toast().text, "🦊 🦄 +3");
    assert!(reply.toast_undo_entry().is_some());
    assert!(reply.pots().contains(&Pot {
        name: "Kid 1".into(),
        pebbles: 3
    }));
    assert!(closes_sheet(&reply));
    // The section is redrawn with the new row at the end, already ✓ for Kid 1.
    let mut expected = before.clone();
    expected.push("Feed the fish".into());
    assert_eq!(oob_rows(&reply, "daily_chores"), Some(expected.clone()));

    let board = app.board().await;
    assert_eq!(board.items("daily_chores"), expected);
    assert_eq!(board.done_by("Feed the fish"), ["Kid 1"]);
    assert_eq!(app.pot("Kid 1").await, 3);
    let today = app.today_list("Kid 1").await.entries();
    assert_eq!(today.len(), 1);
    assert_eq!(
        (today[0].name.as_str(), today[0].amount),
        ("Feed the fish", 3)
    );
    // The new row has a face for everyone and its value as dots.
    let row = select(&board.0, "[data-item-name=\"Feed the fish\"]");
    assert_eq!(row[0].value().attr("data-value"), Some("3"));
    assert_eq!(
        board.faces_per_row("daily_chores").last().unwrap(),
        &board.member_ids()
    );
}

#[tokio::test]
async fn each_section_creates_its_own_kind_of_item() {
    let app = TestApp::start().await;
    for (section, name) in [
        ("daily_chores", "Feed the fish"),
        ("good_deeds", "Helped a neighbour"),
        ("occasional_chores", "Wash the windows"),
    ] {
        let reply = something_else(&app, Ask::new(section, "Kid 2", name)).await;
        assert_eq!(reply.toast().kind, "earned", "{section}");
        assert_eq!(oob_rows(&reply, section).unwrap().last().unwrap(), name);
        let board = app.board().await;
        for other in SECTIONS {
            assert_eq!(
                board.items(other).contains(&name.to_string()),
                other == section,
                "{name} in {other}"
            );
        }
    }
    assert_eq!(app.pot("Kid 2").await, 9);
}

#[tokio::test]
async fn a_new_occasional_chore_works_like_the_others() {
    let app = TestApp::start().await;
    let section = "occasional_chores";
    something_else(&app, Ask::new(section, "Kid 1", "Wash the windows")).await;
    assert_eq!(app.board().await.done_by("Wash the windows"), ["Kid 1"]);

    // Once per person per day, like any chore; someone else can still do it.
    let again = app.earn_chore("Kid 1", "Wash the windows").await;
    assert_eq!(again.toast().refusal.as_deref(), Some("already_done_today"));
    let mom = app.earn_chore("Mom", "Wash the windows").await;
    assert_eq!(mom.toast().text, "🦉 🦄 +3");

    // It's found by the Other jobs search.
    let (_, rows) = app
        .get(&format!(
            "{}?q=window",
            pebblepot::web::OCCASIONAL_ROWS_PATH
        ))
        .await;
    let rows = Fragment(Html::parse_fragment(&rows));
    assert_eq!(rows.item_names(), ["Wash the windows"]);
}

#[tokio::test]
async fn a_new_good_deed_gets_a_badge_and_repeats_like_the_others() {
    let app = TestApp::start().await;
    something_else(&app, Ask::new("good_deeds", "Kid 2", "Helped a neighbour")).await;
    let board = app.board().await;
    let kid2 = board.member_id("Kid 2");
    let badge = select(
        &board.0,
        &format!("[data-item-name=\"Helped a neighbour\"] [data-badge-for=\"{kid2}\"]"),
    );
    assert_eq!(badge[0].value().attr("data-count"), Some("1"));
    assert!(board.done_by("Helped a neighbour").is_empty());

    // A face tap moments later is a double-tap; a few seconds later it counts again.
    let item = board.item_id("Helped a neighbour");
    let tap = format!("member={kid2}&item={item}");
    let (_, body) = app
        .post_form(pebblepot::web::EARN_GOOD_DEED_PATH, &tap)
        .await;
    let refused = Fragment(Html::parse_fragment(&body));
    assert_eq!(refused.toast().refusal.as_deref(), Some("repeat_tap"));
    app.clock.advance(SignedDuration::from_secs(3));
    let (_, body) = app
        .post_form(pebblepot::web::EARN_GOOD_DEED_PATH, &tap)
        .await;
    assert_eq!(Fragment(Html::parse_fragment(&body)).toast().kind, "earned");
    assert_eq!(app.pot("Kid 2").await, 6);
}

#[tokio::test]
async fn with_keep_on_the_board_off_only_a_log_entry_is_written() {
    let app = TestApp::start().await;
    let before = all_items(&app.board().await);
    let reply = something_else(
        &app,
        Ask {
            picture: "🎻",
            value: Some(4),
            keep: false,
            ..Ask::new("good_deeds", "Kid 1", "Played violin for Grandma")
        },
    )
    .await;

    assert_eq!(reply.toast().kind, "earned");
    assert_eq!(reply.toast().text, "🦊 🎻 +4");
    assert!(reply.toast_undo_entry().is_some());
    assert!(closes_sheet(&reply));
    assert!(reply.item_names().contains(&"Shared nicely".to_string()));
    assert!(
        !reply
            .item_names()
            .contains(&"Played violin for Grandma".to_string())
    );

    let board = app.board().await;
    assert_eq!(all_items(&board), before, "no row appears");
    assert_eq!(app.pot("Kid 1").await, 4);
    let today = app.today_list("Kid 1").await.entries();
    assert_eq!(today.len(), 1);
    assert_eq!(
        (today[0].name.as_str(), today[0].amount),
        ("Played violin for Grandma", 4)
    );
    // The picture is the entry's own copy too.
    let (_, list) = app
        .get(&format!(
            "{}?member={}",
            pebblepot::web::TODAY_PATH,
            board.member_id("Kid 1")
        ))
        .await;
    let list = Html::parse_fragment(&list);
    let picture = select(&list, "[data-entry] .picture");
    assert_eq!(picture[0].text().collect::<String>(), "🎻");
}

#[tokio::test]
async fn a_one_off_from_any_section_adds_no_row() {
    let app = TestApp::start().await;
    let before = all_items(&app.board().await);
    for section in SECTIONS {
        let reply = something_else(
            &app,
            Ask {
                keep: false,
                ..Ask::new(section, "Dad", "Fixed the bike")
            },
        )
        .await;
        assert_eq!(reply.toast().kind, "earned", "{section}");
        app.clock.advance(SignedDuration::from_secs(5));
    }
    assert_eq!(all_items(&app.board().await), before);
    assert_eq!(app.pot("Dad").await, 9);
    assert_eq!(app.today_list("Dad").await.names().len(), 3);
}

#[tokio::test]
async fn the_pebble_value_defaults_to_1() {
    let app = TestApp::start().await;
    let kept = something_else(
        &app,
        Ask {
            value: None,
            ..Ask::new("daily_chores", "Kid 1", "Feed the fish")
        },
    )
    .await;
    assert_eq!(kept.toast().text, "🦊 🦄 +1");
    let one_off = something_else(
        &app,
        Ask {
            value: None,
            keep: false,
            ..Ask::new("good_deeds", "Kid 2", "Sang a song")
        },
    )
    .await;
    assert_eq!(one_off.toast().text, "🐻 🦄 +1");

    let board = app.board().await;
    let row = select(&board.0, "[data-item-name=\"Feed the fish\"]");
    assert_eq!(row[0].value().attr("data-value"), Some("1"));
    assert_eq!(app.pot("Kid 1").await, 1);
    assert_eq!(app.pot("Kid 2").await, 1);
}

#[tokio::test]
async fn a_blank_picture_gets_a_sparkle() {
    let app = TestApp::start().await;
    let reply = something_else(
        &app,
        Ask {
            picture: "  ",
            ..Ask::new("good_deeds", "Kid 1", "Kind words to a friend")
        },
    )
    .await;
    assert_eq!(reply.toast().text, "🦊 ✨ +3");
}

#[tokio::test]
async fn a_kept_earn_can_be_undone_from_the_toast_and_the_new_row_stays() {
    let app = TestApp::start().await;
    let reply = something_else(&app, Ask::new("daily_chores", "Kid 1", "Feed the fish")).await;
    let undone = app.undo(&reply.toast_undo_entry().unwrap()).await;

    assert_eq!(undone.toast().kind, "undone");
    assert_eq!(undone.done_faces(), Vec::<String>::new());
    assert_eq!(undone.item_names(), ["Feed the fish"]);
    assert_eq!(app.pot("Kid 1").await, 0);
    assert!(app.today_list("Kid 1").await.entries().is_empty());
    let board = app.board().await;
    assert!(
        board
            .items("daily_chores")
            .contains(&"Feed the fish".to_string())
    );
    assert!(board.done_by("Feed the fish").is_empty());
    // It can be earned again straight away.
    let again = app.earn_chore("Kid 1", "Feed the fish").await;
    assert_eq!(again.toast().kind, "earned");
}

#[tokio::test]
async fn a_one_off_can_be_undone_from_the_today_list() {
    let app = TestApp::start().await;
    something_else(
        &app,
        Ask {
            keep: false,
            ..Ask::new("occasional_chores", "Mom", "Cleared the gutters")
        },
    )
    .await;
    let entry = app.today_list("Mom").await.entry("Cleared the gutters");
    let undone = app.undo(&entry).await;

    assert_eq!(undone.toast().kind, "undone");
    assert!(undone.item_names().is_empty());
    assert!(undone.today_list().unwrap().entries().is_empty());
    assert_eq!(app.pot("Mom").await, 0);
}

#[tokio::test]
async fn something_else_from_yesterday_cant_be_undone() {
    let app = TestApp::start().await;
    // 23:50 in Rome on the 14th.
    app.clock.set("2026-10-14T21:50:00Z".parse().unwrap());
    let kept = something_else(&app, Ask::new("daily_chores", "Kid 1", "Feed the fish")).await;
    let one_off = something_else(
        &app,
        Ask {
            keep: false,
            ..Ask::new("good_deeds", "Kid 1", "Sang a song")
        },
    )
    .await;
    app.clock.advance(SignedDuration::from_mins(20));

    for reply in [kept, one_off] {
        let refused = app.undo(&reply.toast_undo_entry().unwrap()).await;
        assert_eq!(refused.toast().refusal.as_deref(), Some("not_undoable"));
    }
    assert_eq!(app.pot("Kid 1").await, 6);
    // A new day: the kept chore is on the board, ready to be done again.
    let board = app.board().await;
    assert!(board.done_by("Feed the fish").is_empty());
    let today = app.earn_chore("Kid 1", "Feed the fish").await;
    assert_eq!(today.toast().kind, "earned");
}

#[tokio::test]
async fn a_double_tap_on_give_pebbles_counts_once() {
    let app = TestApp::start().await;
    let before = app.board().await.items("daily_chores").len();
    something_else(&app, Ask::new("daily_chores", "Kid 1", "Feed the fish")).await;
    app.clock.advance(SignedDuration::from_millis(500));
    let again = something_else(&app, Ask::new("daily_chores", "Kid 1", "Feed the fish")).await;

    assert_eq!(again.toast().refusal.as_deref(), Some("repeat_tap"));
    assert!(!closes_sheet(&again), "a refusal leaves the sheet open");
    assert_eq!(app.board().await.items("daily_chores").len(), before + 1);
    assert_eq!(app.pot("Kid 1").await, 3);

    // The same one-off for someone else, or later on, is a separate earn.
    let one_off = |member| Ask {
        keep: false,
        ..Ask::new("good_deeds", member, "Sang a song")
    };
    something_else(&app, one_off("Kid 2")).await;
    let kid1 = something_else(&app, one_off("Kid 1")).await;
    assert_eq!(kid1.toast().kind, "earned");
    let quick = something_else(&app, one_off("Kid 1")).await;
    assert_eq!(quick.toast().refusal.as_deref(), Some("repeat_tap"));
    app.clock.advance(SignedDuration::from_secs(3));
    let later = something_else(&app, one_off("Kid 1")).await;
    assert_eq!(later.toast().kind, "earned");
    assert_eq!(app.pot("Kid 1").await, 9);
}

#[tokio::test]
async fn a_missing_name_or_a_value_below_1_is_refused_and_nothing_is_written() {
    let app = TestApp::start().await;
    let before = all_items(&app.board().await);
    for (ask, refusal) in [
        (Ask::new("daily_chores", "Kid 1", "   "), "name_needed"),
        (
            Ask {
                value: Some(0),
                ..Ask::new("daily_chores", "Kid 1", "Feed the fish")
            },
            "bad_value",
        ),
        (
            Ask {
                value: Some(-5),
                keep: false,
                ..Ask::new("good_deeds", "Kid 1", "Shouted")
            },
            "bad_value",
        ),
        (
            Ask {
                value: Some(1000),
                ..Ask::new("good_deeds", "Kid 1", "Typo")
            },
            "bad_value",
        ),
    ] {
        let reply = something_else(&app, ask).await;
        assert_eq!(reply.toast().refusal.as_deref(), Some(refusal));
        assert!(!closes_sheet(&reply));
    }
    assert_eq!(all_items(&app.board().await), before);
    assert_eq!(app.pot("Kid 1").await, 0);
    assert!(app.today_list("Kid 1").await.entries().is_empty());
}

#[tokio::test]
async fn nobody_picked_or_an_unknown_person_is_refused() {
    let app = TestApp::start().await;
    let before = all_items(&app.board().await);
    let ask = Ask::new("daily_chores", "Kid 1", "Feed the fish");
    for member in ["", "999"] {
        let (status, body) = app
            .post_form(SOMETHING_ELSE_PATH, &form(&ask, member))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let reply = Fragment(Html::parse_fragment(&body));
        assert_eq!(reply.toast().refusal.as_deref(), Some("unknown_member"));
    }
    assert_eq!(all_items(&app.board().await), before);
}

#[tokio::test]
async fn an_unknown_section_is_not_found() {
    let app = TestApp::start().await;
    let (status, _) = app
        .get(&format!("{SOMETHING_ELSE_PATH}?section=rewards"))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let member = app.board().await.member_id("Kid 1");
    let ask = Ask::new("rewards", "Kid 1", "Free money");
    let (status, _) = app
        .post_form(SOMETHING_ELSE_PATH, &form(&ask, &member))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.pot("Kid 1").await, 0);
}

#[tokio::test]
async fn the_reply_keeps_the_other_jobs_search() {
    let app = TestApp::start().await;
    let member = app.board().await.member_id("Kid 1");
    let ask = Ask::new("occasional_chores", "Kid 1", "Wash the windows");
    let (status, body) = app
        .post_form(
            SOMETHING_ELSE_PATH,
            &format!("{}&q=wash", form(&ask, &member)),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let reply = Fragment(Html::parse_fragment(&body));
    let rows = oob_rows(&reply, "occasional_chores").unwrap();
    assert!(rows.contains(&"Wash the windows".to_string()));
    assert!(
        rows.iter().all(|n| n.to_lowercase().contains("wash")),
        "{rows:?}"
    );
}

#[tokio::test]
async fn other_devices_see_the_new_row_on_their_next_refresh() {
    let app = TestApp::start().await;
    something_else(&app, Ask::new("good_deeds", "Kid 2", "Helped a neighbour")).await;
    let (status, body) = app.get(pebblepot::web::BOARD_REFRESH_PATH).await;
    assert_eq!(status, StatusCode::OK);
    let refresh = Fragment(Html::parse_fragment(&body));
    assert!(
        oob_rows(&refresh, "good_deeds")
            .unwrap()
            .contains(&"Helped a neighbour".to_string())
    );
    assert!(refresh.pots().contains(&Pot {
        name: "Kid 2".into(),
        pebbles: 3
    }));
}

#[tokio::test]
async fn cross_site_something_else_posts_are_rejected() {
    let app = TestApp::start().await;
    let member = app.board().await.member_id("Kid 1");
    let ask = Ask::new("daily_chores", "Kid 1", "Feed the fish");
    let (status, _) = app
        .send(
            Request::post(SOMETHING_ELSE_PATH)
                .header(header::HOST, HOST)
                .header(header::ORIGIN, "http://evil.example")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form(&ask, &member)))
                .unwrap(),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(app.pot("Kid 1").await, 0);
    assert!(!all_items(&app.board().await).contains(&"Feed the fish".to_string()));
}
