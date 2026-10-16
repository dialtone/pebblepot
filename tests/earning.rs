mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{Fragment, HOST, TestApp};
use jiff::SignedDuration;
use scraper::Html;

#[tokio::test]
async fn tapping_a_face_on_a_daily_chore_earns_its_pebbles() {
    let app = TestApp::start().await;
    let reply = app.earn_chore("Kid 2", "Tidy toys").await;

    assert_eq!(reply.toast().kind, "earned");
    assert_eq!(reply.toast().text, "🐻 🧸 +2");
    assert_eq!(app.pot("Kid 2").await, 2);
    assert_eq!(app.pot("Kid 1").await, 0);
    assert_eq!(app.board().await.done_by("Tidy toys"), ["Kid 2"]);
}

#[tokio::test]
async fn tapping_a_face_on_an_occasional_chore_earns_its_pebbles() {
    let app = TestApp::start().await;
    app.earn_chore("Dad", "Rake leaves").await;

    assert_eq!(app.pot("Dad").await, 4);
    assert_eq!(app.board().await.done_by("Rake leaves"), ["Dad"]);
}

#[tokio::test]
async fn earnings_add_up_in_the_pot() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    app.earn_chore("Kid 1", "Tidy toys").await;
    app.earn_chore("Kid 1", "Help cook").await;

    assert_eq!(app.pot("Kid 1").await, 1 + 2 + 3);
}

#[tokio::test]
async fn the_same_chore_counts_only_once_per_person_per_day() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Make bed").await;
    let again = app.earn_chore("Kid 2", "Make bed").await;

    assert_eq!(again.toast().kind, "refused");
    assert_eq!(again.toast().refusal.as_deref(), Some("already_done_today"));
    assert_eq!(again.toast().text, "Already done today ✓");
    assert_eq!(app.pot("Kid 2").await, 1);
}

#[tokio::test]
async fn occasional_chores_also_count_only_once_a_day() {
    let app = TestApp::start().await;
    app.earn_chore("Mom", "Fold laundry").await;
    let again = app.earn_chore("Mom", "Fold laundry").await;

    assert_eq!(again.toast().refusal.as_deref(), Some("already_done_today"));
    assert_eq!(app.pot("Mom").await, 3);
}

#[tokio::test]
async fn different_people_can_each_do_the_same_chore() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    app.earn_chore("Kid 2", "Make bed").await;

    assert_eq!(app.board().await.done_by("Make bed"), ["Kid 1", "Kid 2"]);
    assert_eq!(app.pot("Kid 1").await, 1);
    assert_eq!(app.pot("Kid 2").await, 1);
}

#[tokio::test]
async fn ticks_reset_at_local_midnight_in_the_household_time_zone() {
    // The test household is in Europe/Rome; the test clock starts at 10:00 there on 14 October.
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;

    // 23:59 in Rome (21:59 UTC): still the same day.
    app.clock.set("2026-10-14T21:59:00Z".parse().unwrap());
    assert_eq!(app.board().await.done_by("Make bed"), ["Kid 1"]);
    let refused = app.earn_chore("Kid 1", "Make bed").await;
    assert_eq!(
        refused.toast().refusal.as_deref(),
        Some("already_done_today")
    );

    // Two minutes later it's 00:01 on 15 October in Rome, though still the 14th in UTC.
    app.clock.advance(SignedDuration::from_mins(2));
    assert!(app.board().await.done_by("Make bed").is_empty());
    let earned = app.earn_chore("Kid 1", "Make bed").await;
    assert_eq!(earned.toast().kind, "earned");
    assert_eq!(app.pot("Kid 1").await, 2);
}

#[tokio::test]
async fn the_reply_carries_the_updated_row_and_pots() {
    let app = TestApp::start().await;
    let reply = app.earn_chore("Kid 1", "Get dressed").await;

    assert_eq!(reply.item_names(), ["Get dressed"]);
    let html = reply.0.html();
    assert!(html.contains(r#"id="pots""#) && html.contains("hx-swap-oob"));
    assert!(html.contains(r#"data-pot="1""#));
}

#[tokio::test]
async fn the_other_jobs_search_filters_by_name() {
    let app = TestApp::start().await;
    let search = |q: &'static str| {
        let app = &app;
        async move {
            let (status, body) = app
                .get(&format!("{}?q={q}", pebblepot::web::OCCASIONAL_ROWS_PATH))
                .await;
            assert_eq!(status, StatusCode::OK);
            Fragment(Html::parse_fragment(&body)).item_names()
        }
    };

    assert_eq!(search("RAKE").await, ["Rake leaves"]);
    assert_eq!(search("table").await, ["Set the table", "Wipe the table"]);
    assert_eq!(search("").await.len(), 18);
    assert!(search("spaceship").await.is_empty());
}

#[tokio::test]
async fn search_results_keep_todays_ticks() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Rake leaves").await;
    let (_, body) = app
        .get(&format!("{}?q=rake", pebblepot::web::OCCASIONAL_ROWS_PATH))
        .await;
    assert!(body.contains("data-done"));
}

#[tokio::test]
async fn good_deeds_cannot_be_earned_as_chores() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let (member, item) = (board.member_id("Kid 1"), board.item_id("Shared nicely"));
    let (status, _) = app
        .post_form(
            pebblepot::web::EARN_CHORE_PATH,
            &format!("member={member}&item={item}"),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.pot("Kid 1").await, 0);
}

#[tokio::test]
async fn cross_site_earn_posts_are_rejected() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let (member, item) = (board.member_id("Kid 1"), board.item_id("Make bed"));
    let request = Request::post(pebblepot::web::EARN_CHORE_PATH)
        .header(header::HOST, HOST)
        .header(header::ORIGIN, "http://evil.example")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("member={member}&item={item}")))
        .unwrap();
    let (status, _) = app.send(request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(app.pot("Kid 1").await, 0);
}
