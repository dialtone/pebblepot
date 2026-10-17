//! ✏️ Family members in board edit mode: tap a pot to rename, re-face, re-colour or retire that
//! person, or ＋ Add someone. Pots and log entries never change.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{BoardPage, Fragment, HOST, TestApp};
use pebblepot::web::{
    EARN_CHORE_PATH, MEMBER_EDIT_PATH, MEMBER_NEW_PATH, MEMBER_RETIRE_PATH, REDEEM_PATH,
    SOMETHING_ELSE_PATH, SPEND_PATH, TODAY_PATH,
};
use scraper::{ElementRef, Html, Selector};
use tower::ServiceExt;

const SECTIONS: [&str; 3] = ["daily_chores", "good_deeds", "occasional_chores"];

fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).collect()
}

fn attr(el: ElementRef<'_>, name: &str) -> String {
    el.value()
        .attr(name)
        .unwrap_or_else(|| panic!("no {name} on {}", el.html()))
        .to_string()
}

fn text(el: ElementRef<'_>) -> String {
    el.text().collect::<String>().trim().to_string()
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

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn names(board: &BoardPage) -> Vec<String> {
    board.pots().into_iter().map(|p| p.name).collect()
}

async fn get_ok(app: &TestApp, path: &str) -> Html {
    let (status, body) = app.get(path).await;
    assert_eq!(status, StatusCode::OK, "{path}: {body}");
    Html::parse_fragment(&body)
}

async fn post_ok(app: &TestApp, path: &str, form: &str) -> Fragment {
    let (status, body) = app.post_form(path, form).await;
    assert_eq!(status, StatusCode::OK, "{path} {form}: {body}");
    Fragment(Html::parse_fragment(&body))
}

/// The board in edit mode, reached through the ✏️ Edit toggle.
async fn edit_board(app: &TestApp) -> BoardPage {
    let board = app.board().await;
    let href = attr(select(&board.0, "[data-edit-toggle]")[0], "href");
    let (status, body) = app.get(&href).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    BoardPage(Html::parse_document(&body))
}

/// Taps `member`'s pot in edit mode: their ✏️ sheet.
async fn open_member(app: &TestApp, member: &str) -> Html {
    let board = edit_board(app).await;
    let id = board.member_id(member);
    let pot = select(&board.0, &format!("[data-pots] [data-member=\"{id}\"]"))[0];
    let pencil = pot
        .select(&Selector::parse("[data-edit-member]").unwrap())
        .next()
        .expect("no ✏️ on the pot");
    assert_eq!(attr(pencil, "data-edit-member"), id);
    get_ok(app, &attr(pencil, "hx-get")).await
}

fn member_form(name: &str, face: &str, colour: &str) -> String {
    format!(
        "name={}&face={}&colour={}",
        enc(name),
        enc(face),
        enc(colour)
    )
}

async fn add(app: &TestApp, name: &str, face: &str, colour: &str, role: &str) -> Fragment {
    post_ok(
        app,
        MEMBER_NEW_PATH,
        &format!("{}&role={role}", member_form(name, face, colour)),
    )
    .await
}

async fn change(app: &TestApp, member: &str, name: &str, face: &str, colour: &str) -> Fragment {
    let id = app.board().await.member_id(member);
    post_ok(
        app,
        MEMBER_EDIT_PATH,
        &format!("member={id}&{}", member_form(name, face, colour)),
    )
    .await
}

async fn retire(app: &TestApp, member: &str) -> Fragment {
    let id = app.board().await.member_id(member);
    post_ok(app, MEMBER_RETIRE_PATH, &format!("member={id}")).await
}

fn closes_sheet(reply: &Fragment) -> bool {
    select(&reply.0, "#sheet[hx-swap-oob]")
        .first()
        .is_some_and(|s| s.inner_html().trim().is_empty())
}

/// Every face button for `member_id` on the board's rows: (face, colour style, label).
fn faces_for(board: &BoardPage, member_id: &str) -> Vec<(String, String, String)> {
    select(
        &board.0,
        &format!("[data-item] [data-face-for=\"{member_id}\"]"),
    )
    .into_iter()
    .map(|f| (text(f), attr(f, "style"), attr(f, "aria-label")))
    .collect()
}

fn all_rows(board: &BoardPage) -> usize {
    SECTIONS.iter().map(|s| board.items(s).len()).sum()
}

/// Names offered by a picker (Spend's "whose pebbles?" or Something else's "who did it?").
async fn spend_picker(app: &TestApp) -> Vec<String> {
    let sheet = get_ok(app, SPEND_PATH).await;
    select(&sheet, "[data-spend-for]")
        .into_iter()
        .map(|p| attr(p, "data-member-name"))
        .collect()
}

async fn something_else_picker(app: &TestApp) -> Vec<String> {
    let sheet = get_ok(app, &format!("{SOMETHING_ELSE_PATH}?section=daily_chores")).await;
    select(&sheet, "[data-member-choice]")
        .into_iter()
        .map(|c| text(ElementRef::wrap(c.parent().unwrap()).unwrap()))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// In edit mode

#[tokio::test]
async fn in_edit_mode_every_pot_opens_that_persons_sheet() {
    let app = TestApp::start().await;
    let board = edit_board(&app).await;
    let ids = board.member_ids();
    assert_eq!(ids.len(), 4);
    for id in &ids {
        let pot = select(&board.0, &format!("[data-pots] [data-member=\"{id}\"]"))[0];
        let pencil = pot
            .select(&Selector::parse("[data-edit-member]").unwrap())
            .next()
            .expect("no ✏️ on the pot");
        assert_eq!(
            attr(pencil, "hx-get"),
            format!("{MEMBER_EDIT_PATH}?member={id}")
        );
        assert_eq!(attr(pencil, "hx-target"), "#sheet");
    }
    let add = select(&board.0, "[data-pots] [data-new-member]");
    assert_eq!(add.len(), 1, "no ＋ Add someone");
    assert_eq!(attr(add[0], "hx-get"), MEMBER_NEW_PATH);
}

#[tokio::test]
async fn a_family_members_sheet_starts_with_them_as_they_are() {
    let app = TestApp::start().await;
    let id = app.board().await.member_id("Kid 1");
    let sheet = open_member(&app, "Kid 1").await;
    assert_eq!(
        attr(
            select(&sheet, "[data-member-sheet]")[0],
            "data-member-sheet"
        ),
        id
    );
    assert_eq!(attr(select(&sheet, "form")[0], "hx-post"), MEMBER_EDIT_PATH);
    assert_eq!(attr(select(&sheet, "input[name=member]")[0], "value"), id);
    assert_eq!(
        attr(select(&sheet, "input[name=name]")[0], "value"),
        "Kid 1"
    );
    assert_eq!(attr(select(&sheet, "input[name=face]")[0], "value"), "🦊");
    let checked = select(&sheet, "[data-colour-choice][checked]");
    assert_eq!(checked.len(), 1);
    assert_eq!(attr(checked[0], "value"), "#f59e0b");
    // No kid-or-parent choice when changing someone, and 🗑 Retire asks first.
    assert!(select(&sheet, "[data-role-choice]").is_empty());
    let retire = select(&sheet, "[data-retire-member]");
    assert_eq!(attr(retire[0], "hx-post"), MEMBER_RETIRE_PATH);
    assert!(retire[0].value().attr("hx-confirm").is_some());
}

#[tokio::test]
async fn the_add_someone_sheet_starts_as_a_kid_with_a_fresh_colour() {
    let app = TestApp::start().await;
    let sheet = get_ok(&app, MEMBER_NEW_PATH).await;
    assert_eq!(attr(select(&sheet, "form")[0], "hx-post"), MEMBER_NEW_PATH);
    assert_eq!(attr(select(&sheet, "input[name=name]")[0], "value"), "");
    assert_eq!(attr(select(&sheet, "input[name=face]")[0], "value"), "🙂");
    assert!(select(&sheet, "input[name=member], [data-retire-member]").is_empty());
    // The four starting colours are taken, so the next one in the palette.
    let checked = select(&sheet, "[data-colour-choice][checked]");
    assert_eq!(attr(checked[0], "value"), "#8b5cf6");
    let role = select(&sheet, "[data-role-choice][checked]");
    assert_eq!(attr(role[0], "value"), "kid");
    let roles: Vec<String> = select(&sheet, "[data-role-choice]")
        .into_iter()
        .map(|r| attr(r, "value"))
        .collect();
    assert_eq!(roles, s(&["kid", "parent"]));
}

// ---------------------------------------------------------------------------------------------
// ＋ Add someone

#[tokio::test]
async fn someone_new_gets_a_pot_and_a_face_on_every_row() {
    let mut app = TestApp::start().await;
    let reply = add(&app, "Grandma", "👵", "#8b5cf6", "parent").await;
    assert_eq!(reply.toast().kind, "added");
    assert!(closes_sheet(&reply));
    assert_eq!(
        reply
            .pots()
            .into_iter()
            .map(|p| (p.name, p.pebbles))
            .collect::<Vec<_>>(),
        vec![
            ("Kid 1".to_string(), 0),
            ("Kid 2".to_string(), 0),
            ("Mom".to_string(), 0),
            ("Dad".to_string(), 0),
            ("Grandma".to_string(), 0),
        ]
    );
    let board = app.board().await;
    let id = board.member_id("Grandma");
    assert_eq!(
        names(&board),
        s(&["Kid 1", "Kid 2", "Mom", "Dad", "Grandma"])
    );
    for section in SECTIONS {
        for faces in board.faces_per_row(section) {
            assert_eq!(faces.last(), Some(&id), "{section}");
        }
    }
    let faces = faces_for(&board, &id);
    assert_eq!(faces.len(), all_rows(&board));
    assert!(faces.iter().all(|(face, style, label)| face == "👵"
        && style.contains("#8b5cf6")
        && label == "Grandma"));
    assert_eq!(spend_picker(&app).await.last().unwrap(), "Grandma");
    assert!(
        something_else_picker(&app)
            .await
            .last()
            .unwrap()
            .contains("Grandma")
    );

    // She earns like anyone, and she's still there after a restart (with no re-seeding).
    app.earn_chore("Grandma", "Help cook").await;
    assert_eq!(app.pot("Grandma").await, 3);
    app.restart().await;
    assert_eq!(
        names(&app.board().await),
        s(&["Kid 1", "Kid 2", "Mom", "Dad", "Grandma"])
    );
    assert_eq!(app.pot("Grandma").await, 3);
}

#[tokio::test]
async fn someone_new_with_no_face_or_role_is_a_smiling_kid() {
    let app = TestApp::start().await;
    let (status, body) = app
        .post_form(MEMBER_NEW_PATH, &member_form("Cousin", " ", "#0EA5E9"))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let board = app.board().await;
    let faces = faces_for(&board, &board.member_id("Cousin"));
    assert_eq!(faces[0].0, "🙂");
    assert!(faces[0].1.contains("#0ea5e9"), "{}", faces[0].1);
}

// ---------------------------------------------------------------------------------------------
// ✏️ Change

#[tokio::test]
async fn renaming_refacing_and_recolouring_show_everywhere() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    app.earn_chore("Kid 1", "Tidy toys").await;
    let id = app.board().await.member_id("Kid 1");
    let reply = change(&app, "Kid 1", "Ada", "🐱", "#ef4444").await;
    assert_eq!(reply.toast().kind, "edited");
    assert!(closes_sheet(&reply));
    assert_eq!(reply.pots()[0].name, "Ada");

    let board = app.board().await;
    assert_eq!(names(&board), s(&["Ada", "Kid 2", "Mom", "Dad"]));
    assert_eq!(board.member_id("Ada"), id);
    let pot = select(&board.0, &format!("[data-pots] [data-member=\"{id}\"]"))[0];
    let face = pot
        .select(&Selector::parse(".face").unwrap())
        .next()
        .unwrap();
    assert_eq!(
        (text(face), attr(face, "style")),
        ("🐱".to_string(), "--c: #ef4444".to_string())
    );
    // The faces on the rows (✓s included: they keep the colour).
    for (face, style, label) in faces_for(&board, &id) {
        assert!(style.contains("#ef4444"), "{style}");
        assert!(label.starts_with("Ada"), "{label}");
        assert!(face == "🐱" || face == "✓", "{face}");
    }
    assert_eq!(board.done_by("Make bed"), s(&["Ada"]));
    // Spend, Something else and the today list.
    assert_eq!(spend_picker(&app).await[0], "Ada");
    assert!(something_else_picker(&app).await[0].contains("Ada"));
    let today = get_ok(&app, &format!("{TODAY_PATH}?member={id}")).await;
    assert!(text(select(&today, ".sheet-head")[0]).contains("Ada's day"));
    assert_eq!(text(select(&today, ".sheet-head .face")[0]), "🐱");
    // The pot and the log are as they were.
    assert_eq!(app.pot("Ada").await, 3);
    assert_eq!(
        app.today_list("Ada").await.names(),
        s(&["Tidy toys", "Make bed"])
    );
    // And the next earn's toast has the new face.
    let reply = app.earn_chore("Ada", "Get dressed").await;
    assert!(
        reply.toast().text.starts_with("🐱"),
        "{}",
        reply.toast().text
    );
}

#[tokio::test]
async fn a_family_member_needs_a_name_and_a_colour() {
    let app = TestApp::start().await;
    let cases = [
        ("  ", "#ef4444", "name_needed"),
        ("Ada", "red", "bad_colour"),
        ("Ada", "", "bad_colour"),
        ("Ada", "#12345", "bad_colour"),
        ("Ada", "#gggggg", "bad_colour"),
        ("Ada", "#ef4444; background: url(x)", "bad_colour"),
    ];
    for (name, colour, refusal) in cases {
        for reply in [
            change(&app, "Kid 1", name, "🐱", colour).await,
            add(&app, name, "🐱", colour, "kid").await,
        ] {
            assert_eq!(
                reply.toast().refusal.as_deref(),
                Some(refusal),
                "{name:?} {colour:?}"
            );
            assert!(reply.pots().is_empty(), "a refusal keeps the sheet open");
            assert!(!closes_sheet(&reply));
        }
    }
    assert_eq!(
        names(&app.board().await),
        s(&["Kid 1", "Kid 2", "Mom", "Dad"])
    );
    let (status, _) = app
        .post_form(
            MEMBER_NEW_PATH,
            &format!("{}&role=pet", member_form("Rex", "🐶", "#ef4444")),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        names(&app.board().await),
        s(&["Kid 1", "Kid 2", "Mom", "Dad"])
    );
}

// ---------------------------------------------------------------------------------------------
// 🗑 Retire

#[tokio::test]
async fn retiring_someone_takes_their_pot_and_faces_off_the_board() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    app.earn_chore("Kid 2", "Make bed").await;
    let id = app.board().await.member_id("Kid 2");
    let reply = retire(&app, "Kid 2").await;
    assert_eq!(reply.toast().kind, "retired");
    assert!(closes_sheet(&reply));
    assert_eq!(
        reply.pots().into_iter().map(|p| p.name).collect::<Vec<_>>(),
        s(&["Kid 1", "Mom", "Dad"])
    );

    let board = app.board().await;
    assert_eq!(names(&board), s(&["Kid 1", "Mom", "Dad"]));
    assert!(faces_for(&board, &id).is_empty(), "no faces left");
    for section in SECTIONS {
        assert!(
            board.faces_per_row(section).iter().all(|f| f.len() == 3),
            "{section}"
        );
    }
    assert_eq!(board.done_by("Make bed"), s(&["Kid 1"]));
    assert!(!spend_picker(&app).await.contains(&"Kid 2".to_string()));
    assert!(
        !something_else_picker(&app)
            .await
            .iter()
            .any(|n| n.contains("Kid 2"))
    );
    let edit = edit_board(&app).await;
    assert!(select(&edit.0, &format!("[data-edit-member=\"{id}\"]")).is_empty());
    // Everyone else's pot is as it was.
    assert_eq!(app.pot("Kid 1").await, 1);
}

#[tokio::test]
async fn a_retired_family_member_cant_earn_spend_or_be_changed() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let id = board.member_id("Kid 2");
    let chore = board.item_id("Make bed");
    retire(&app, "Kid 2").await;
    let reply = post_ok(&app, EARN_CHORE_PATH, &format!("member={id}&item={chore}")).await;
    assert_eq!(reply.toast().refusal.as_deref(), Some("unknown_member"));
    assert!(reply.done_faces().is_empty());
    let sheet = get_ok(&app, &format!("{SPEND_PATH}?edit=1")).await;
    let sticker = attr(
        select(&sheet, "[data-reward-name=\"Sticker\"]")[0],
        "data-reward",
    );
    let reply = post_ok(&app, REDEEM_PATH, &format!("member={id}&item={sticker}")).await;
    assert_eq!(reply.toast().refusal.as_deref(), Some("unknown_member"));
    for path in [
        format!("{TODAY_PATH}?member={id}"),
        format!("{SPEND_PATH}?member={id}"),
        format!("{MEMBER_EDIT_PATH}?member={id}"),
        format!("{MEMBER_EDIT_PATH}?member=999999"),
    ] {
        let (status, _) = app.get(&path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
    }
    for (path, form) in [
        (
            MEMBER_EDIT_PATH,
            format!("member={id}&{}", member_form("Back", "🐻", "#3b82f6")),
        ),
        (MEMBER_EDIT_PATH, member_form("No id", "🐻", "#3b82f6")),
        (MEMBER_RETIRE_PATH, format!("member={id}")),
        (MEMBER_RETIRE_PATH, "member=999999".to_string()),
    ] {
        let (status, _) = app.post_form(path, &form).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path} {form}");
    }
    assert_eq!(names(&app.board().await), s(&["Kid 1", "Mom", "Dad"]));
}

#[tokio::test]
async fn a_retired_family_members_log_entries_are_kept() {
    let app = TestApp::start().await;
    let earned = app.earn_chore("Kid 2", "Tidy toys").await;
    let entry = earned.toast_undo_entry().unwrap();
    retire(&app, "Kid 2").await;
    // The entry is still in the log: today it can still be undone, rather than "already undone".
    let reply = app.undo(&entry).await;
    assert_eq!(reply.toast().kind, "undone");
    let reply = app.undo(&entry).await;
    assert_eq!(reply.toast().refusal.as_deref(), Some("unknown_entry"));
}

// ---------------------------------------------------------------------------------------------
// Edges

#[tokio::test]
async fn family_edits_reach_every_open_board() {
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
    change(&app, "Kid 1", "Ada", "🐱", "#ef4444").await;
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
    .expect("no changed event after a family edit");
    assert!(frame.contains("data: "), "{frame}");
}

#[tokio::test]
async fn cross_site_family_posts_are_rejected() {
    let app = TestApp::start().await;
    let id = app.board().await.member_id("Kid 1");
    for (path, form) in [
        (
            MEMBER_EDIT_PATH,
            format!("member={id}&{}", member_form("Hacked", "💀", "#000000")),
        ),
        (
            MEMBER_NEW_PATH,
            format!("{}&role=kid", member_form("Hacked", "💀", "#000000")),
        ),
        (MEMBER_RETIRE_PATH, format!("member={id}")),
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
    assert_eq!(
        names(&app.board().await),
        s(&["Kid 1", "Kid 2", "Mom", "Dad"])
    );
}
