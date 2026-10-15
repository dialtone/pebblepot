mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{HOST, Pot, TestApp};

// The starting content (.scratch/pebblepot-v1/starting-content.md), as the board should show it.
const DAILY_CHORES: &[&str] = &[
    "Make bed",
    "Get dressed",
    "Brush teeth (morning)",
    "Brush teeth (bedtime)",
    "Clear my plate",
    "Tidy toys",
    "Dirty clothes in the hamper",
    "Unpack my bag",
    "Bath or shower time, no fuss",
];
const GOOD_DEEDS: &[&str] = &[
    "Shared nicely",
    "Helped my brother or sister",
    "Helped without being asked",
    "Listened the first time",
    "Said please and thank you",
    "Said sorry",
    "Kind words",
    "Stayed calm when upset",
    "Was brave",
    "Tried a new food",
    "Played well together",
];
const OCCASIONAL_CHORES: &[&str] = &[
    "Set the table",
    "Wipe the table",
    "Help cook",
    "Unload the dishwasher",
    "Match the socks",
    "Fold laundry",
    "Put my clothes away",
    "Sweep the floor",
    "Vacuum a room",
    "Tidy my whole room",
    "Water the plants",
    "Sort the recycling",
    "Take out the trash",
    "Tidy the shoes",
    "Carry the shopping",
    "Rake leaves",
    "Help wash the car",
    "Make a card for someone",
];

fn starting_pots() -> Vec<Pot> {
    ["Kid 1", "Kid 2", "Mom", "Dad"]
        .map(|name| Pot {
            name: name.into(),
            pebbles: 0,
        })
        .into()
}

#[tokio::test]
async fn first_start_shows_the_family_with_empty_pots() {
    let app = TestApp::start().await;
    let board = app.board().await;
    assert_eq!(board.pots(), starting_pots());
    assert!(board.has_spend_button());
}

#[tokio::test]
async fn first_start_shows_every_section_in_order_with_the_starting_content() {
    let app = TestApp::start().await;
    let board = app.board().await;
    assert_eq!(
        board.section_order(),
        ["daily_chores", "good_deeds", "occasional_chores"]
    );
    assert_eq!(board.items("daily_chores"), DAILY_CHORES);
    assert_eq!(board.items("good_deeds"), GOOD_DEEDS);
    assert_eq!(board.items("occasional_chores"), OCCASIONAL_CHORES);
}

#[tokio::test]
async fn every_row_has_a_face_button_for_every_family_member() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let members = board.member_ids();
    for section in ["daily_chores", "good_deeds", "occasional_chores"] {
        for faces in board.faces_per_row(section) {
            assert_eq!(faces, members, "in {section}");
        }
    }
}

#[tokio::test]
async fn restarting_does_not_seed_again() {
    let mut app = TestApp::start().await;
    app.restart().await;
    app.restart().await;
    let board = app.board().await;
    assert_eq!(board.pots(), starting_pots());
    assert_eq!(board.items("daily_chores"), DAILY_CHORES);
    assert_eq!(board.items("good_deeds"), GOOD_DEEDS);
    assert_eq!(board.items("occasional_chores"), OCCASIONAL_CHORES);
}

#[tokio::test]
async fn cross_origin_posts_are_rejected() {
    let app = TestApp::start().await;
    let post = |origin: &str| {
        Request::post("/")
            .header(header::HOST, HOST)
            .header(header::ORIGIN, origin)
            .body(Body::empty())
            .unwrap()
    };
    let (status, _) = app.send(post("http://evil.example")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    // Same origin gets past the protection (there's no POST route here yet).
    let (status, _) = app.send(post(&format!("http://{HOST}"))).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn htmx_and_the_stylesheet_are_served() {
    let app = TestApp::start().await;
    let (status, body) = app.get(pebblepot::web::HTMX_PATH).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with("var htmx="));
    let (status, _) = app.get(pebblepot::web::CSS_PATH).await;
    assert_eq!(status, StatusCode::OK);
}
