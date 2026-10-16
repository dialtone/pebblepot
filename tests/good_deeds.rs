//! 💛 Kind things: repeatable good deeds, the 2-second repeat-tap guard and the count badges.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{BoardPage, Fragment, HOST, Pot, TestApp};
use jiff::SignedDuration;
use scraper::{Html, Selector};

/// Taps `member`'s face on a good deed's row. Returns the response fragment.
async fn earn_good_deed(app: &TestApp, member: &str, deed: &str) -> Fragment {
    let board = app.board().await;
    let (member, item) = (board.member_id(member), board.item_id(deed));
    let (status, body) = app
        .post_form(
            pebblepot::web::EARN_GOOD_DEED_PATH,
            &format!("member={member}&item={item}"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fragment(Html::parse_fragment(&body))
}

/// The badge on `member`'s face on this good deed's row: (today's count, the entry a tap undoes).
fn badge(html: &Html, deed: &str, member_id: &str) -> Option<(i64, String)> {
    let sel = Selector::parse(&format!(
        "[data-item-name=\"{deed}\"] [data-badge-for=\"{member_id}\"]"
    ))
    .unwrap();
    html.select(&sel).next().map(|b| {
        (
            b.value().attr("data-count").unwrap().parse().unwrap(),
            b.value().attr("data-undo-entry").unwrap().to_string(),
        )
    })
}

/// Today's count on `member`'s badge for this good deed on the board; 0 when there is no badge.
async fn count(app: &TestApp, deed: &str, member: &str) -> i64 {
    let board = app.board().await;
    badge(&board.0, deed, &board.member_id(member)).map_or(0, |(n, _)| n)
}

/// Taps `member`'s badge on this good deed's row (it must be showing).
async fn tap_badge(app: &TestApp, deed: &str, member: &str) -> Fragment {
    let board = app.board().await;
    let (_, entry) = badge(&board.0, deed, &board.member_id(member)).expect("no badge to tap");
    let item = board.item_id(deed);
    let (status, body) = app
        .post_form(
            pebblepot::web::UNDO_PATH,
            &format!("entry={entry}&item={item}"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Fragment(Html::parse_fragment(&body))
}

fn secs(n: i64) -> SignedDuration {
    SignedDuration::from_secs(n)
}

#[tokio::test]
async fn faces_on_kind_things_rows_earn_the_good_deed() {
    let app = TestApp::start().await;
    let reply = earn_good_deed(&app, "Kid 1", "Shared nicely").await;

    assert_eq!(reply.toast().kind, "earned");
    assert_eq!(reply.toast().text, "🦊 🤝 +2");
    assert!(reply.toast_undo_entry().is_some());
    assert_eq!(reply.item_names(), ["Shared nicely"]);
    assert!(reply.pots().contains(&Pot {
        name: "Kid 1".into(),
        pebbles: 2
    }));
    assert_eq!(app.pot("Kid 1").await, 2);
    assert_eq!(count(&app, "Shared nicely", "Kid 1").await, 1);
    // A good deed never turns into a ✓.
    assert!(app.board().await.done_by("Shared nicely").is_empty());
}

#[tokio::test]
async fn a_good_deed_earned_twice_more_than_2_seconds_apart_counts_twice() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 2", "Shared nicely").await;
    app.clock.advance(secs(3));
    let second = earn_good_deed(&app, "Kid 2", "Shared nicely").await;

    assert_eq!(second.toast().kind, "earned");
    assert_eq!(app.pot("Kid 2").await, 4);
    assert_eq!(count(&app, "Shared nicely", "Kid 2").await, 2);
    assert_eq!(
        app.today_list("Kid 2").await.names(),
        ["Shared nicely", "Shared nicely"]
    );
    // The reply's row already shows the new count.
    let board = app.board().await;
    let kid2 = board.member_id("Kid 2");
    assert_eq!(badge(&second.0, "Shared nicely", &kid2).unwrap().0, 2);
}

#[tokio::test]
async fn an_identical_tap_within_2_seconds_is_refused_and_the_pot_is_unchanged() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 1", "Was brave").await;
    app.clock.advance(SignedDuration::from_millis(400));
    let again = earn_good_deed(&app, "Kid 1", "Was brave").await;

    assert_eq!(again.toast().kind, "refused");
    assert_eq!(again.toast().refusal.as_deref(), Some("repeat_tap"));
    assert_eq!(again.toast().text, "Already counted 👍");
    assert_eq!(again.toast_undo_entry(), None);
    assert_eq!(app.pot("Kid 1").await, 2);
    assert_eq!(count(&app, "Was brave", "Kid 1").await, 1);
    assert_eq!(app.today_list("Kid 1").await.names(), ["Was brave"]);
}

#[tokio::test]
async fn the_repeat_window_is_2_seconds_by_the_clock() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Mom", "Kind words").await;

    // Exactly 2 seconds later is still within the window.
    app.clock.advance(secs(2));
    let at_two = earn_good_deed(&app, "Mom", "Kind words").await;
    assert_eq!(at_two.toast().refusal.as_deref(), Some("repeat_tap"));

    // A refused tap doesn't restart the window: just past 2 seconds after the earn counts.
    app.clock.advance(SignedDuration::from_millis(1));
    let after = earn_good_deed(&app, "Mom", "Kind words").await;
    assert_eq!(after.toast().kind, "earned");
    assert_eq!(app.pot("Mom").await, 2);
    assert_eq!(count(&app, "Kind words", "Mom").await, 2);
}

#[tokio::test]
async fn the_window_counts_from_the_latest_earn() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 2", "Tried a new food").await;
    app.clock.advance(secs(5));
    earn_good_deed(&app, "Kid 2", "Tried a new food").await;
    app.clock.advance(secs(1));
    let double_tap = earn_good_deed(&app, "Kid 2", "Tried a new food").await;

    assert_eq!(double_tap.toast().refusal.as_deref(), Some("repeat_tap"));
    assert_eq!(app.pot("Kid 2").await, 4);
}

#[tokio::test]
async fn the_repeat_guard_is_per_person_and_per_good_deed() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 1", "Played well together").await;
    let other_person = earn_good_deed(&app, "Kid 2", "Played well together").await;
    let other_deed = earn_good_deed(&app, "Kid 1", "Said sorry").await;

    assert_eq!(other_person.toast().kind, "earned");
    assert_eq!(other_deed.toast().kind, "earned");
    assert_eq!(app.pot("Kid 1").await, 2 + 1);
    assert_eq!(app.pot("Kid 2").await, 2);
}

#[tokio::test]
async fn faces_without_a_good_deed_today_show_no_badge() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Dad", "Tried a new food").await;

    let board = app.board().await;
    let badges = Selector::parse("[data-badge-for]").unwrap();
    assert_eq!(board.0.select(&badges).count(), 1);
    for member in ["Kid 1", "Kid 2", "Mom"] {
        assert_eq!(count(&app, "Tried a new food", member).await, 0);
    }
    // Chore rows never show badges.
    app.earn_chore("Dad", "Make bed").await;
    assert_eq!(app.board().await.0.select(&badges).count(), 1);
}

#[tokio::test]
async fn tapping_the_badge_takes_away_the_most_recent_entry() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 2", "Helped my brother or sister").await;
    app.earn_chore("Kid 2", "Make bed").await;
    app.clock.advance(secs(10));
    let latest = earn_good_deed(&app, "Kid 2", "Helped my brother or sister")
        .await
        .toast_undo_entry()
        .unwrap();
    let first = app.today_list("Kid 2").await.entries()[2].entry.clone();

    let reply = tap_badge(&app, "Helped my brother or sister", "Kid 2").await;

    assert_eq!(reply.toast().kind, "undone");
    assert_eq!(reply.toast().text, "🐻 🫶 undone");
    let entries: Vec<String> = app
        .today_list("Kid 2")
        .await
        .entries()
        .into_iter()
        .map(|e| e.entry)
        .collect();
    assert!(!entries.contains(&latest));
    assert!(entries.contains(&first));
    assert_eq!(app.pot("Kid 2").await, 2 + 1);
    assert_eq!(count(&app, "Helped my brother or sister", "Kid 2").await, 1);
    // The reply redraws the row out of band with the lower count, and the pots.
    let kid2 = app.board().await.member_id("Kid 2");
    assert_eq!(reply.item_names(), ["Helped my brother or sister"]);
    assert_eq!(
        badge(&reply.0, "Helped my brother or sister", &kid2),
        Some((1, first))
    );
    assert!(reply.pots().contains(&Pot {
        name: "Kid 2".into(),
        pebbles: 3
    }));
}

#[tokio::test]
async fn the_badge_disappears_at_zero() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 1", "Listened the first time").await;
    app.clock.advance(secs(5));
    earn_good_deed(&app, "Kid 1", "Listened the first time").await;

    tap_badge(&app, "Listened the first time", "Kid 1").await;
    let reply = tap_badge(&app, "Listened the first time", "Kid 1").await;

    let kid1 = app.board().await.member_id("Kid 1");
    assert_eq!(badge(&reply.0, "Listened the first time", &kid1), None);
    assert_eq!(count(&app, "Listened the first time", "Kid 1").await, 0);
    assert_eq!(app.pot("Kid 1").await, 0);
    assert!(app.today_list("Kid 1").await.entries().is_empty());
    // The face is still there to earn it again.
    let faces = BoardPage(app.board().await.0).faces_per_row("good_deeds");
    assert!(faces.iter().all(|row| row.contains(&kid1)));
}

#[tokio::test]
async fn a_double_tapped_badge_takes_away_only_one() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Mom", "Stayed calm when upset").await;
    app.clock.advance(secs(5));
    earn_good_deed(&app, "Mom", "Stayed calm when upset").await;
    let board = app.board().await;
    let (_, entry) = badge(&board.0, "Stayed calm when upset", &board.member_id("Mom")).unwrap();

    // Both taps carry the entry the badge showed; the second finds it already undone.
    app.undo(&entry).await;
    let second = app.undo(&entry).await;

    assert_eq!(second.toast().refusal.as_deref(), Some("unknown_entry"));
    assert_eq!(app.pot("Mom").await, 2);
    assert_eq!(count(&app, "Stayed calm when upset", "Mom").await, 1);
}

#[tokio::test]
async fn good_deeds_are_in_the_today_list_and_can_be_undone_there() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Dad", "Helped without being asked").await;
    app.earn_chore("Dad", "Make bed").await;

    let list = app.today_list("Dad").await;
    assert_eq!(list.names(), ["Make bed", "Helped without being asked"]);
    assert_eq!(list.entries()[1].amount, 3);

    let reply = app.undo(&list.entry("Helped without being asked")).await;

    assert_eq!(reply.toast().kind, "undone");
    assert_eq!(reply.today_list().unwrap().names(), ["Make bed"]);
    // The row is redrawn too, so its badge goes away on the board underneath.
    assert_eq!(reply.item_names(), ["Helped without being asked"]);
    let dad = app.board().await.member_id("Dad");
    assert_eq!(badge(&reply.0, "Helped without being asked", &dad), None);
    assert_eq!(app.pot("Dad").await, 1);
    assert_eq!(count(&app, "Helped without being asked", "Dad").await, 0);
}

#[tokio::test]
async fn the_toast_undo_works_for_good_deeds() {
    let app = TestApp::start().await;
    let entry = earn_good_deed(&app, "Kid 2", "Said please and thank you")
        .await
        .toast_undo_entry()
        .unwrap();

    app.undo(&entry).await;

    assert_eq!(app.pot("Kid 2").await, 0);
    assert_eq!(count(&app, "Said please and thank you", "Kid 2").await, 0);
}

#[tokio::test]
async fn badges_reset_at_local_midnight() {
    // The test household is in Europe/Rome; 23:59:59 there on 14 October.
    let app = TestApp::start().await;
    app.clock.set("2026-10-14T21:59:59Z".parse().unwrap());
    earn_good_deed(&app, "Kid 1", "Shared nicely").await;
    assert_eq!(count(&app, "Shared nicely", "Kid 1").await, 1);

    // Two seconds later it's tomorrow: the badge is gone and the pot keeps the pebbles.
    app.clock.advance(secs(3));
    assert_eq!(count(&app, "Shared nicely", "Kid 1").await, 0);
    assert_eq!(app.pot("Kid 1").await, 2);
    earn_good_deed(&app, "Kid 1", "Shared nicely").await;
    assert_eq!(count(&app, "Shared nicely", "Kid 1").await, 1);
    assert_eq!(app.pot("Kid 1").await, 4);
}

#[tokio::test]
async fn the_live_refetch_shows_the_badges() {
    let app = TestApp::start().await;
    earn_good_deed(&app, "Kid 2", "Was brave").await;
    let kid2 = app.board().await.member_id("Kid 2");

    let (status, body) = app.get(pebblepot::web::BOARD_REFRESH_PATH).await;

    assert_eq!(status, StatusCode::OK);
    let refresh = Html::parse_fragment(&body);
    assert_eq!(badge(&refresh, "Was brave", &kid2).map(|(n, _)| n), Some(1));
}

#[tokio::test]
async fn chores_cannot_be_earned_as_good_deeds() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let (member, item) = (board.member_id("Kid 1"), board.item_id("Make bed"));
    let (status, _) = app
        .post_form(
            pebblepot::web::EARN_GOOD_DEED_PATH,
            &format!("member={member}&item={item}"),
        )
        .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.pot("Kid 1").await, 0);
}

#[tokio::test]
async fn cross_site_good_deed_posts_are_rejected() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let (member, item) = (board.member_id("Kid 1"), board.item_id("Was brave"));
    let request = Request::post(pebblepot::web::EARN_GOOD_DEED_PATH)
        .header(header::HOST, HOST)
        .header(header::ORIGIN, "http://evil.example")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("member={member}&item={item}")))
        .unwrap();
    let (status, _) = app.send(request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(app.pot("Kid 1").await, 0);
}
