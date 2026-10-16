mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{HOST, Pot, TestApp, TodayEntry};
use jiff::SignedDuration;

#[tokio::test]
async fn tapping_a_tick_undoes_that_chore_for_that_person() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Tidy toys").await;
    app.earn_chore("Kid 2", "Tidy toys").await;
    let tick = app.board().await.done_entry("Tidy toys", "Kid 1");

    let reply = app.undo(&tick).await;

    assert_eq!(reply.toast().kind, "undone");
    assert_eq!(reply.toast().text, "🦊 🧸 undone");
    assert_eq!(app.pot("Kid 1").await, 0);
    assert_eq!(app.pot("Kid 2").await, 2);
    assert_eq!(app.board().await.done_by("Tidy toys"), ["Kid 2"]);
}

#[tokio::test]
async fn the_undo_reply_carries_the_row_with_the_face_back_and_the_pots() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    let tick = app.board().await.done_entry("Make bed", "Kid 1");

    let reply = app.undo(&tick).await;

    assert_eq!(reply.item_names(), ["Make bed"]);
    assert!(reply.done_faces().is_empty());
    assert!(reply.pots().contains(&Pot {
        name: "Kid 1".into(),
        pebbles: 0
    }));
    // Everything is swapped in out of band: the tap may have come from a ✓, the toast or a sheet.
    assert!(reply.0.html().contains(r#"id="row-"#));
}

#[tokio::test]
async fn after_undo_the_chore_can_be_earned_again() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Make bed").await;
    let tick = app.board().await.done_entry("Make bed", "Kid 2");
    app.undo(&tick).await;

    let again = app.earn_chore("Kid 2", "Make bed").await;
    assert_eq!(again.toast().kind, "earned");
    assert_eq!(app.pot("Kid 2").await, 1);
}

#[tokio::test]
async fn the_toast_undo_removes_the_entry_it_reported() {
    let app = TestApp::start().await;
    app.earn_chore("Dad", "Rake leaves").await;
    let earned = app.earn_chore("Dad", "Fold laundry").await;
    let entry = earned.toast_undo_entry().expect("toast offers undo");

    let reply = app.undo(&entry).await;

    assert_eq!(reply.toast().kind, "undone");
    assert_eq!(reply.item_names(), ["Fold laundry"]);
    assert_eq!(app.pot("Dad").await, 4);
    let board = app.board().await;
    assert_eq!(board.done_by("Rake leaves"), ["Dad"]);
    assert!(board.done_by("Fold laundry").is_empty());
}

#[tokio::test]
async fn refused_toasts_offer_no_undo() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    let again = app.earn_chore("Kid 1", "Make bed").await;

    assert_eq!(again.toast().kind, "refused");
    assert_eq!(again.toast_undo_entry(), None);
}

#[tokio::test]
async fn undoing_twice_is_refused_and_changes_nothing() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    let entry = app
        .earn_chore("Kid 1", "Tidy toys")
        .await
        .toast_undo_entry()
        .unwrap();
    app.undo(&entry).await;

    let again = app.undo(&entry).await;
    assert_eq!(again.toast().refusal.as_deref(), Some("unknown_entry"));
    assert_eq!(app.pot("Kid 1").await, 1);
}

#[tokio::test]
async fn the_today_list_shows_all_of_todays_entries_most_recent_first() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Make bed").await;
    app.earn_chore("Kid 2", "Rake leaves").await;
    app.earn_chore("Kid 1", "Tidy toys").await;

    let list = app.today_list("Kid 2").await;
    let entries: Vec<(String, i64)> = list
        .entries()
        .into_iter()
        .map(|e| (e.name, e.amount))
        .collect();
    assert_eq!(
        entries,
        [("Rake leaves".to_string(), 4), ("Make bed".to_string(), 1)]
    );
    assert!(app.today_list("Mom").await.entries().is_empty());
}

#[tokio::test]
async fn undo_from_the_today_list_removes_the_entry_and_redraws_the_list() {
    let app = TestApp::start().await;
    app.earn_chore("Mom", "Make bed").await;
    app.earn_chore("Mom", "Fold laundry").await;
    let entry = app.today_list("Mom").await.entry("Make bed");

    let reply = app.undo(&entry).await;

    let list = reply.today_list().expect("reply redraws the today list");
    assert_eq!(list.names(), ["Fold laundry"]);
    assert_eq!(app.today_list("Mom").await.names(), ["Fold laundry"]);
    assert_eq!(app.pot("Mom").await, 3);
    assert!(app.board().await.done_by("Make bed").is_empty());
}

#[tokio::test]
async fn the_today_list_for_someone_not_on_the_board_is_not_found() {
    let app = TestApp::start().await;
    let (status, _) = app
        .get(&format!("{}?member=999", pebblepot::web::TODAY_PATH))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn yesterdays_entries_cannot_be_undone_and_are_not_in_the_today_list() {
    // The test household is in Europe/Rome; the test clock starts at 10:00 there on 14 October.
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    let yesterday = app.today_list("Kid 1").await.entry("Make bed");

    // 23:59 in Rome: still the same day, so it's still undoable (checked without undoing).
    app.clock.set("2026-10-14T21:59:00Z".parse().unwrap());
    assert_eq!(
        app.today_list("Kid 1").await.entries(),
        [TodayEntry {
            entry: yesterday.clone(),
            name: "Make bed".into(),
            amount: 1
        }]
    );

    // 00:01 on 15 October in Rome, though still the 14th in UTC.
    app.clock.advance(SignedDuration::from_mins(2));
    app.earn_chore("Kid 1", "Tidy toys").await;
    assert_eq!(app.today_list("Kid 1").await.names(), ["Tidy toys"]);

    let refused = app.undo(&yesterday).await;
    assert_eq!(refused.toast().kind, "refused");
    assert_eq!(refused.toast().refusal.as_deref(), Some("not_undoable"));
    assert_eq!(refused.toast().text, "Only today's entries can be undone");
    assert_eq!(app.pot("Kid 1").await, 1 + 2);
}

#[tokio::test]
async fn a_stale_tick_from_yesterday_is_refused_and_its_row_redrawn() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Make bed").await;
    let board = app.board().await;
    let (tick, item) = (
        board.done_entry("Make bed", "Kid 2"),
        board.item_id("Make bed"),
    );

    // The board was left open overnight and the ✓ is tapped the next morning.
    app.clock.advance(SignedDuration::from_hours(24));
    let (status, body) = app
        .post_form(
            pebblepot::web::UNDO_PATH,
            &format!("entry={tick}&item={item}"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let reply = common::Fragment(scraper::Html::parse_fragment(&body));
    assert_eq!(reply.toast().refusal.as_deref(), Some("not_undoable"));
    assert_eq!(reply.item_names(), ["Make bed"]);
    assert!(reply.done_faces().is_empty());
    assert_eq!(app.pot("Kid 2").await, 1);
}

#[tokio::test]
async fn cross_site_undo_posts_are_rejected() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    let tick = app.board().await.done_entry("Make bed", "Kid 1");
    let request = Request::post(pebblepot::web::UNDO_PATH)
        .header(header::HOST, HOST)
        .header(header::ORIGIN, "http://evil.example")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("entry={tick}")))
        .unwrap();
    let (status, _) = app.send(request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(app.pot("Kid 1").await, 1);
}
