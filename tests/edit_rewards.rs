//! ✏️ The Spend sheet's edit mode: rewards are added, changed, reordered and retired there. None of
//! it touches the log, the pots or a goal's stored price.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{Fragment, HOST, TestApp};
use jiff::SignedDuration;
use pebblepot::web::{
    BOARD_REFRESH_PATH, GOAL_GIVE_UP_PATH, GOAL_GOT_IT_PATH, GOAL_SET_PATH, REDEEM_PATH,
    REWARD_EDIT_PATH, REWARD_NEW_PATH, REWARD_REORDER_PATH, REWARD_RETIRE_PATH, SPEND_PATH,
};
use scraper::{ElementRef, Html, Selector};

fn select<'a>(html: &'a Html, selector: &str) -> Vec<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).collect()
}

fn attr(el: ElementRef<'_>, name: &str) -> String {
    el.value()
        .attr(name)
        .unwrap_or_else(|| panic!("no {name} on {}", el.html()))
        .to_string()
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

/// A reward tile: (name, picture, price).
fn tile(el: ElementRef<'_>) -> (String, String, i64) {
    let picture = el
        .select(&Selector::parse(".em").unwrap())
        .next()
        .unwrap()
        .text()
        .collect::<String>();
    (
        attr(el, "data-reward-name"),
        picture.trim().to_string(),
        attr(el, "data-price").parse().unwrap(),
    )
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

/// The ordinary Spend sheet for `member`.
async fn spend(app: &TestApp, member: &str) -> Html {
    let id = app.board().await.member_id(member);
    get_ok(app, &format!("{SPEND_PATH}?member={id}")).await
}

/// Reward names in Spend order, from `member`'s ordinary Spend sheet.
async fn rewards(app: &TestApp) -> Vec<String> {
    let sheet = spend(app, "Mom").await;
    select(&sheet, "[data-reward]")
        .into_iter()
        .map(|t| attr(t, "data-reward-name"))
        .collect()
}

/// The Spend sheet in edit mode, reached through its ✏️ Edit toggle from Kid 1's sheet.
async fn edit_sheet(app: &TestApp) -> Html {
    let sheet = spend(app, "Kid 1").await;
    let toggle = select(&sheet, "[data-spend-edit-toggle]");
    assert_eq!(toggle.len(), 1, "no ✏️ Edit on the Spend sheet");
    get_ok(app, &attr(toggle[0], "hx-get")).await
}

async fn reward_id(app: &TestApp, name: &str) -> String {
    let sheet = get_ok(app, &format!("{SPEND_PATH}?edit=1")).await;
    let tile = select(&sheet, &format!("[data-reward-name=\"{name}\"]"));
    assert_eq!(tile.len(), 1, "no {name} tile");
    attr(tile[0], "data-reward")
}

fn reward_form(name: &str, picture: &str, price: &str) -> String {
    format!(
        "name={}&picture={}&price={}",
        enc(name),
        enc(picture),
        enc(price)
    )
}

async fn add_reward(app: &TestApp, name: &str, picture: &str, price: &str) -> Fragment {
    post_ok(app, REWARD_NEW_PATH, &reward_form(name, picture, price)).await
}

async fn change_reward(
    app: &TestApp,
    reward: &str,
    name: &str,
    picture: &str,
    price: &str,
) -> Fragment {
    let id = reward_id(app, reward).await;
    post_ok(
        app,
        REWARD_EDIT_PATH,
        &format!("item={id}&{}", reward_form(name, picture, price)),
    )
    .await
}

async fn retire_reward(app: &TestApp, reward: &str) -> Fragment {
    let id = reward_id(app, reward).await;
    post_ok(app, REWARD_RETIRE_PATH, &format!("item={id}")).await
}

/// Drops a dragged tile: posts the new order (reward names), as reorder.js does.
async fn reorder(app: &TestApp, names: &[&str]) -> Fragment {
    let mut ids = Vec::new();
    for name in names {
        ids.push(reward_id(app, name).await);
    }
    post_ok(
        app,
        REWARD_REORDER_PATH,
        &format!("section=rewards&order={}&q=", ids.join("%2C")),
    )
    .await
}

/// The reward tiles a reply redraws (out of band), by name.
fn redrawn_tiles(reply: &Fragment) -> Option<Vec<String>> {
    let tiles = select(&reply.0, "[data-reward-tiles]");
    let tiles = tiles.first()?;
    Some(
        tiles
            .select(&Selector::parse("[data-reward]").unwrap())
            .map(|t| attr(t, "data-reward-name"))
            .collect(),
    )
}

/// The reply closes the reward's ✏️ sheet (an empty `#sheet-over`) and redraws the Spend sheet's
/// tiles out of band, leaving the Spend sheet itself (in `#sheet`) open.
fn closes_reward_sheet(reply: &Fragment) -> bool {
    let over = select(&reply.0, "#sheet-over[hx-swap-oob]");
    over.len() == 1
        && over[0].inner_html().trim().is_empty()
        && select(&reply.0, "#reward-tiles[hx-swap-oob]").len() == 1
        && select(&reply.0, "#sheet").is_empty()
}

/// Kid 1 earns 1 + 2 + 3 = 6 pebbles.
async fn kid_1_with_six_pebbles() -> TestApp {
    let app = TestApp::start().await;
    app.earn_chore("Kid 1", "Make bed").await;
    app.earn_chore("Kid 1", "Tidy toys").await;
    app.earn_chore("Kid 1", "Help cook").await;
    assert_eq!(app.pot("Kid 1").await, 6);
    app
}

/// Kid 1 has 6 pebbles and saves for 🍿 Movie night with popcorn (30): pot −24.
async fn kid_1_saving_for_movie_night() -> TestApp {
    let app = kid_1_with_six_pebbles().await;
    let kid = app.board().await.member_id("Kid 1");
    let movie = reward_id(&app, "Movie night with popcorn").await;
    let reply = post_ok(&app, GOAL_SET_PATH, &format!("member={kid}&item={movie}")).await;
    assert_eq!(reply.toast().kind, "goal_set");
    assert_eq!(app.pot("Kid 1").await, -24);
    app
}

/// Kid 1's pot card while saving: (goal name, saved, to go).
async fn goal_card(app: &TestApp) -> Option<(String, i64, i64)> {
    let board = app.board().await;
    let id = board.member_id("Kid 1");
    let card = select(&board.0, &format!("[data-pots] [data-member=\"{id}\"]"))[0];
    card.value().attr("data-goal").map(|goal| {
        (
            goal.to_string(),
            attr(card, "data-saved").parse().unwrap(),
            attr(card, "data-to-go").parse().unwrap(),
        )
    })
}

// ---------------------------------------------------------------------------------------------
// The toggle

#[tokio::test]
async fn the_spend_sheets_edit_toggle_gives_every_reward_a_pencil_a_handle_and_retire() {
    let app = TestApp::start().await;
    let normal = rewards(&app).await;
    let sheet = edit_sheet(&app).await;
    assert_eq!(select(&sheet, "[data-spend-editing]").len(), 1);
    let tiles = select(&sheet, "[data-reward]");
    let names: Vec<String> = tiles.iter().map(|t| attr(*t, "data-reward-name")).collect();
    assert_eq!(names, normal, "same rewards, same order");
    for t in tiles {
        let id = attr(t, "data-reward");
        assert_eq!(attr(t, "data-item"), id, "reorder.js reads data-item");
        let one = |marker: &str| {
            t.select(&Selector::parse(marker).unwrap())
                .next()
                .unwrap_or_else(|| panic!("no {marker} on {id}"))
        };
        assert_eq!(attr(one("[data-edit-reward]"), "data-edit-reward"), id);
        let handle = one("[data-drag-handle]");
        assert_eq!(attr(handle, "data-reorder-section"), "rewards");
        assert_eq!(attr(handle, "data-reorder-path"), REWARD_REORDER_PATH);
        let retire = one("[data-retire-reward]");
        assert_eq!(attr(retire, "hx-post"), REWARD_RETIRE_PATH);
        assert!(
            retire.value().attr("hx-confirm").is_some(),
            "retire asks first"
        );
        // Tapping a tile while editing never redeems.
        assert_ne!(t.value().attr("hx-post"), Some(REDEEM_PATH));
    }
    assert!(select(&sheet, "[hx-post=\"/redeem\"], [data-save-for]").is_empty());
    let new = select(&sheet, "[data-new-reward]");
    assert_eq!(new.len(), 1, "no ＋ New reward tile");
    // The live refresh sends this along.
    assert_eq!(select(&sheet, "[data-spend-edit]").len(), 1);
    assert!(select(&sheet, "[data-spend-member]").is_empty());
}

#[tokio::test]
async fn done_goes_back_to_the_ordinary_spend_sheet() {
    let app = TestApp::start().await;
    let kid = app.board().await.member_id("Kid 1");
    let sheet = edit_sheet(&app).await;
    let done = select(&sheet, "[data-spend-edit-done]");
    assert_eq!(
        attr(done[0], "hx-get"),
        format!("{SPEND_PATH}?member={kid}")
    );
    let back = get_ok(&app, &attr(done[0], "hx-get")).await;
    assert_eq!(
        attr(select(&back, "[data-spend-sheet]")[0], "data-spend-sheet"),
        kid
    );
    assert!(select(&back, "[data-spend-editing], [data-edit-reward]").is_empty());
    // Opened before anyone is picked, Done goes back to "whose pebbles?".
    let sheet = get_ok(&app, &format!("{SPEND_PATH}?edit=1")).await;
    let done = select(&sheet, "[data-spend-edit-done]");
    assert_eq!(attr(done[0], "hx-get"), SPEND_PATH);
}

#[tokio::test]
async fn the_ordinary_spend_sheet_has_no_edit_controls() {
    let app = TestApp::start().await;
    for path in [
        SPEND_PATH.to_string(),
        format!(
            "{SPEND_PATH}?member={}",
            app.board().await.member_id("Kid 1")
        ),
    ] {
        let sheet = get_ok(&app, &path).await;
        assert_eq!(
            select(&sheet, "[data-spend-edit-toggle]").len(),
            1,
            "{path}"
        );
        assert!(
            select(
                &sheet,
                "[data-edit-reward], [data-retire-reward], [data-new-reward], [data-drag-handle]"
            )
            .is_empty(),
            "{path}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// ＋ New reward

#[tokio::test]
async fn the_new_reward_sheet_starts_at_one_pebble() {
    let app = TestApp::start().await;
    let sheet = edit_sheet(&app).await;
    let path = attr(select(&sheet, "[data-new-reward]")[0], "hx-get");
    assert_eq!(path, REWARD_NEW_PATH);
    assert_eq!(
        attr(select(&sheet, "[data-new-reward]")[0], "hx-target"),
        "#sheet-over"
    );
    let new = get_ok(&app, &path).await;
    let form = select(&new, "form")[0];
    assert_eq!(attr(form, "hx-post"), REWARD_NEW_PATH);
    let price = select(&new, "input[name=price]")[0];
    assert_eq!(
        (attr(price, "value"), attr(price, "min"), attr(price, "max")),
        ("1".to_string(), "1".to_string(), "1000".to_string())
    );
    assert_eq!(attr(select(&new, "input[name=name]")[0], "value"), "");
    assert_eq!(attr(select(&new, "input[name=picture]")[0], "value"), "🎁");
    assert!(select(&new, "input[name=item]").is_empty());
}

#[tokio::test]
async fn a_new_reward_joins_the_end_of_spend() {
    let app = kid_1_with_six_pebbles().await;
    let before = rewards(&app).await;
    let reply = add_reward(&app, "Swing ride", "🛝", "4").await;
    assert_eq!(reply.toast().kind, "added");
    assert!(closes_reward_sheet(&reply));
    let mut after = before.clone();
    after.push("Swing ride".to_string());
    assert_eq!(redrawn_tiles(&reply).unwrap(), after);
    assert_eq!(rewards(&app).await, after);

    // It works like any reward: Kid 1 redeems it.
    let sheet = spend(&app, "Kid 1").await;
    let t = select(&sheet, "[data-reward-name=\"Swing ride\"]")[0];
    assert_eq!(tile(t), ("Swing ride".to_string(), "🛝".to_string(), 4));
    assert_eq!(attr(t, "data-state"), "unlocked");
    let kid = app.board().await.member_id("Kid 1");
    let id = attr(t, "data-reward");
    let reply = post_ok(&app, REDEEM_PATH, &format!("member={kid}&item={id}")).await;
    assert_eq!(reply.toast().kind, "redeemed");
    assert_eq!(app.pot("Kid 1").await, 2);
}

#[tokio::test]
async fn a_new_reward_with_no_picture_or_price_is_a_gift_at_one_pebble() {
    let app = TestApp::start().await;
    add_reward(&app, "Surprise", "  ", "").await;
    let sheet = spend(&app, "Kid 1").await;
    let t = select(&sheet, "[data-reward-name=\"Surprise\"]")[0];
    assert_eq!(tile(t), ("Surprise".to_string(), "🎁".to_string(), 1));
}

// ---------------------------------------------------------------------------------------------
// ✏️ Change

#[tokio::test]
async fn the_pencil_sheet_starts_with_the_reward_as_it_is() {
    let app = TestApp::start().await;
    let sheet = edit_sheet(&app).await;
    let id = reward_id(&app, "Big toy").await;
    let pencil = select(&sheet, &format!("[data-edit-reward=\"{id}\"]"))[0];
    let pencil_sheet = get_ok(&app, &attr(pencil, "hx-get")).await;
    assert_eq!(
        attr(
            select(&pencil_sheet, "[data-reward-sheet]")[0],
            "data-reward-sheet"
        ),
        id
    );
    assert_eq!(
        attr(select(&pencil_sheet, "form")[0], "hx-post"),
        REWARD_EDIT_PATH
    );
    assert_eq!(
        attr(select(&pencil_sheet, "input[name=item]")[0], "value"),
        id
    );
    assert_eq!(
        attr(select(&pencil_sheet, "input[name=name]")[0], "value"),
        "Big toy"
    );
    assert_eq!(
        attr(select(&pencil_sheet, "input[name=picture]")[0], "value"),
        "🧸"
    );
    assert_eq!(
        attr(select(&pencil_sheet, "input[name=price]")[0], "value"),
        "100"
    );
    // It opens above the Spend sheet, which stays open underneath.
    assert_eq!(attr(pencil, "hx-target"), "#sheet-over");
}

#[tokio::test]
async fn renaming_repicturing_and_repricing_show_in_spend() {
    let app = kid_1_with_six_pebbles().await;
    let order = rewards(&app).await;
    let reply = change_reward(&app, "Sticker", "Gold sticker", "🌟", "6").await;
    assert_eq!(reply.toast().kind, "edited");
    assert!(closes_reward_sheet(&reply));
    let sheet = spend(&app, "Kid 1").await;
    let t = select(&sheet, "[data-reward-name=\"Gold sticker\"]")[0];
    assert_eq!(tile(t), ("Gold sticker".to_string(), "🌟".to_string(), 6));
    assert!(select(&sheet, "[data-reward-name=\"Sticker\"]").is_empty());
    // It keeps its place.
    let mut renamed = order.clone();
    renamed[0] = "Gold sticker".to_string();
    assert_eq!(rewards(&app).await, renamed);
    // Redeeming takes the new price.
    let kid = app.board().await.member_id("Kid 1");
    let id = attr(t, "data-reward");
    post_ok(&app, REDEEM_PATH, &format!("member={kid}&item={id}")).await;
    assert_eq!(app.pot("Kid 1").await, 0);
    assert_eq!(app.today_list("Kid 1").await.entries()[0].amount, -6);
}

#[tokio::test]
async fn prices_go_from_1_to_1000() {
    let app = TestApp::start().await;
    for price in ["1", "1000", "100"] {
        let reply = change_reward(&app, "Big toy", "Big toy", "🧸", price).await;
        assert_eq!(reply.toast().kind, "edited", "{price}");
        let sheet = spend(&app, "Kid 1").await;
        let t = select(&sheet, "[data-reward-name=\"Big toy\"]")[0];
        assert_eq!(tile(t).2, price.parse::<i64>().unwrap());
    }
    let before = rewards(&app).await;
    for price in ["0", "1001", "-5", "", "lots"] {
        let mut replies = vec![change_reward(&app, "Big toy", "Big toy", "🧸", price).await];
        // A blank price on ＋ New reward means the starting 1 (tested above).
        if !price.is_empty() {
            replies.push(add_reward(&app, "Pony", "🐴", price).await);
        }
        for reply in replies {
            assert_eq!(
                reply.toast().refusal.as_deref(),
                Some("bad_price"),
                "{price:?}"
            );
            assert!(
                redrawn_tiles(&reply).is_none(),
                "a refusal keeps the sheet open"
            );
        }
    }
    assert_eq!(rewards(&app).await, before, "no Pony");
    let sheet = spend(&app, "Kid 1").await;
    assert_eq!(
        tile(select(&sheet, "[data-reward-name=\"Big toy\"]")[0]).2,
        100
    );
}

#[tokio::test]
async fn a_reward_needs_a_name() {
    let app = TestApp::start().await;
    let before = rewards(&app).await;
    for reply in [
        change_reward(&app, "Sticker", "  ", "⭐", "5").await,
        add_reward(&app, "", "🎈", "3").await,
    ] {
        assert_eq!(reply.toast().refusal.as_deref(), Some("name_needed"));
        assert!(redrawn_tiles(&reply).is_none());
    }
    assert_eq!(rewards(&app).await, before);
}

#[tokio::test]
async fn editing_a_reward_never_changes_a_pot_or_a_past_redeem() {
    let app = kid_1_with_six_pebbles().await;
    // Kid 1 spends 5 on a sticker, then the sticker is renamed and re-priced.
    let kid = app.board().await.member_id("Kid 1");
    let sticker = reward_id(&app, "Sticker").await;
    post_ok(&app, REDEEM_PATH, &format!("member={kid}&item={sticker}")).await;
    assert_eq!(app.pot("Kid 1").await, 1);
    change_reward(&app, "Sticker", "Shiny sticker", "✨", "2").await;
    assert_eq!(app.pot("Kid 1").await, 1);
    let today = app.today_list("Kid 1").await.entries();
    assert_eq!((today[0].name.as_str(), today[0].amount), ("Sticker", -5));
    // Undo gives back what was paid, not the new price.
    app.undo(&today[0].entry).await;
    assert_eq!(app.pot("Kid 1").await, 6);
}

// ---------------------------------------------------------------------------------------------
// Reorder

#[tokio::test]
async fn dragging_a_tile_reorders_spend_and_the_order_lasts() {
    let mut app = TestApp::start().await;
    let before = rewards(&app).await;
    let mut order: Vec<&str> = before.iter().map(String::as_str).collect();
    let big_toy = order.iter().position(|n| *n == "Big toy").unwrap();
    let moved = order.remove(big_toy);
    order.insert(0, moved);
    let reply = reorder(&app, &order).await;
    assert_eq!(redrawn_tiles(&reply).unwrap(), s(&order));
    // Just the tiles are swapped in, so the sheet keeps its scroll.
    assert_eq!(select(&reply.0, "#reward-tiles[hx-swap-oob]").len(), 1);
    assert!(select(&reply.0, "#sheet").is_empty());
    assert_eq!(rewards(&app).await, s(&order));
    app.restart().await;
    assert_eq!(rewards(&app).await, s(&order));
}

#[tokio::test]
async fn a_reward_drop_with_a_chore_a_retired_or_a_repeated_tile_snaps_back() {
    let app = TestApp::start().await;
    let before = rewards(&app).await;
    let first = reward_id(&app, &before[0]).await;
    let second = reward_id(&app, &before[1]).await;
    let retired = reward_id(&app, &before[2]).await;
    retire_reward(&app, &before[2]).await;
    let before = rewards(&app).await;
    let chore = app.board().await.item_id("Make bed");
    for order in [
        format!("{second},{chore}"),
        format!("{second},{retired},{first}"),
        format!("{second},{second},{first}"),
        format!("{second},nonsense"),
        "999999".to_string(),
    ] {
        let reply = post_ok(&app, REWARD_REORDER_PATH, &format!("order={}", enc(&order))).await;
        assert_eq!(
            reply.toast().refusal.as_deref(),
            Some("unknown_item"),
            "{order}"
        );
        assert_eq!(
            redrawn_tiles(&reply).unwrap(),
            before,
            "{order}: snaps back"
        );
        assert_eq!(rewards(&app).await, before, "{order}");
    }
    assert_eq!(app.board().await.items("daily_chores")[0], "Make bed");
}

// ---------------------------------------------------------------------------------------------
// 🗑 Retire

#[tokio::test]
async fn a_retired_reward_leaves_spend_and_cant_be_redeemed() {
    let app = kid_1_with_six_pebbles().await;
    let id = reward_id(&app, "Sticker").await;
    let reply = retire_reward(&app, "Sticker").await;
    assert_eq!(reply.toast().kind, "retired");
    assert!(
        !redrawn_tiles(&reply)
            .unwrap()
            .contains(&"Sticker".to_string())
    );
    assert!(!rewards(&app).await.contains(&"Sticker".to_string()));
    let sheet = get_ok(&app, &format!("{SPEND_PATH}?edit=1")).await;
    assert!(select(&sheet, "[data-reward-name=\"Sticker\"]").is_empty());

    let kid = app.board().await.member_id("Kid 1");
    for path in [REDEEM_PATH, GOAL_SET_PATH] {
        let reply = post_ok(&app, path, &format!("member={kid}&item={id}")).await;
        assert_eq!(
            reply.toast().refusal.as_deref(),
            Some("unknown_item"),
            "{path}"
        );
    }
    assert_eq!(app.pot("Kid 1").await, 6);
    // And it can't be changed or retired again.
    let (status, _) = app.get(&format!("{REWARD_EDIT_PATH}?item={id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .post_form(
            REWARD_EDIT_PATH,
            &format!("item={id}&{}", reward_form("Back", "⭐", "5")),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = app
        .post_form(REWARD_RETIRE_PATH, &format!("item={id}"))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_retired_rewards_redeems_keep_their_name_picture_and_price() {
    let app = kid_1_with_six_pebbles().await;
    let kid = app.board().await.member_id("Kid 1");
    let sticker = reward_id(&app, "Sticker").await;
    post_ok(&app, REDEEM_PATH, &format!("member={kid}&item={sticker}")).await;
    retire_reward(&app, "Sticker").await;
    assert_eq!(app.pot("Kid 1").await, 1);
    let today = app.today_list("Kid 1").await;
    let entry = select(&today.0, "[data-entry-name=\"Sticker\"]")[0];
    assert_eq!(attr(entry, "data-amount"), "-5");
    assert!(entry.html().contains("⭐"));
    // Still undoable today.
    app.undo(&today.entry("Sticker")).await;
    assert_eq!(app.pot("Kid 1").await, 6);
}

// ---------------------------------------------------------------------------------------------
// Goals keep their own copy of the reward

#[tokio::test]
async fn repricing_a_goal_reward_leaves_the_goal_and_the_pot_alone() {
    let app = kid_1_saving_for_movie_night().await;
    let before = goal_card(&app).await;
    assert_eq!(
        before,
        Some(("Movie night with popcorn".to_string(), 6, 24))
    );
    change_reward(&app, "Movie night with popcorn", "Movie night", "🎬", "50").await;
    assert_eq!(app.pot("Kid 1").await, -24);
    assert_eq!(goal_card(&app).await, before);
    // Spend shows the goal's own copy and what's saved against its stored price.
    let sheet = spend(&app, "Kid 1").await;
    assert_eq!(
        attr(select(&sheet, "[data-spend-goal]")[0], "data-spend-goal"),
        "Movie night with popcorn"
    );
    assert_eq!(
        attr(select(&sheet, "[data-spendable]")[0], "data-spendable"),
        "6"
    );
    // Giving up refunds the price paid, 30, not the new 50.
    let kid = app.board().await.member_id("Kid 1");
    let reply = post_ok(&app, GOAL_GIVE_UP_PATH, &format!("member={kid}")).await;
    assert_eq!(reply.toast().kind, "goal_given_up");
    assert_eq!(app.pot("Kid 1").await, 6);
}

#[tokio::test]
async fn retiring_a_goal_reward_leaves_the_goal_and_the_pot_alone() {
    let app = kid_1_saving_for_movie_night().await;
    let before = goal_card(&app).await;
    retire_reward(&app, "Movie night with popcorn").await;
    assert!(
        !rewards(&app)
            .await
            .contains(&"Movie night with popcorn".to_string())
    );
    assert_eq!(app.pot("Kid 1").await, -24);
    assert_eq!(goal_card(&app).await, before);

    // Kid 1 earns the 24 back (Helped without being asked, 3 each): Ready, then Got it, as usual.
    let kid = app.board().await.member_id("Kid 1");
    let deed = app.board().await.item_id("Helped without being asked");
    for _ in 0..8 {
        app.clock.advance(SignedDuration::from_secs(3));
        let reply = post_ok(
            &app,
            pebblepot::web::EARN_GOOD_DEED_PATH,
            &format!("member={kid}&item={deed}"),
        )
        .await;
        assert_eq!(reply.toast().kind, "earned");
    }
    assert_eq!(app.pot("Kid 1").await, 0);
    assert_eq!(
        goal_card(&app).await,
        Some(("Movie night with popcorn".to_string(), 30, 0))
    );
    let reply = post_ok(&app, GOAL_GOT_IT_PATH, &format!("member={kid}")).await;
    assert_eq!(reply.toast().kind, "handed_over");
    assert_eq!(goal_card(&app).await, None);
}

#[tokio::test]
async fn giving_up_a_goal_on_a_retired_reward_refunds_its_price() {
    let app = kid_1_saving_for_movie_night().await;
    retire_reward(&app, "Movie night with popcorn").await;
    let kid = app.board().await.member_id("Kid 1");
    post_ok(&app, GOAL_GIVE_UP_PATH, &format!("member={kid}")).await;
    assert_eq!(app.pot("Kid 1").await, 6);
    assert_eq!(goal_card(&app).await, None);
}

// ---------------------------------------------------------------------------------------------
// Edges

#[tokio::test]
async fn chores_and_unknown_ids_arent_edited_as_rewards() {
    let app = TestApp::start().await;
    let chore = app.board().await.item_id("Make bed");
    for id in [chore.as_str(), "999999"] {
        let (status, _) = app.get(&format!("{REWARD_EDIT_PATH}?item={id}")).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
        let (status, _) = app
            .post_form(
                REWARD_EDIT_PATH,
                &format!("item={id}&{}", reward_form("Hacked", "💀", "5")),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
        let (status, _) = app
            .post_form(REWARD_RETIRE_PATH, &format!("item={id}"))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
    }
    let (status, _) = app
        .post_form(REWARD_EDIT_PATH, &reward_form("No id", "💀", "5"))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(app.board().await.items("daily_chores")[0], "Make bed");
}

#[tokio::test]
async fn the_live_refresh_redraws_an_open_edit_sheets_tiles() {
    let app = TestApp::start().await;
    add_reward(&app, "Kite", "🪁", "12").await;
    let (status, body) = app.get(&format!("{BOARD_REFRESH_PATH}?spend_edit=1")).await;
    assert_eq!(status, StatusCode::OK);
    let refresh = Fragment(Html::parse_fragment(&body));
    assert_eq!(select(&refresh.0, "#reward-tiles[hx-swap-oob]").len(), 1);
    assert_eq!(redrawn_tiles(&refresh).unwrap(), rewards(&app).await);
    assert_eq!(redrawn_tiles(&refresh).unwrap().last().unwrap(), "Kite");
    // Without the marker: no tiles.
    let (_, body) = app.get(BOARD_REFRESH_PATH).await;
    assert!(redrawn_tiles(&Fragment(Html::parse_fragment(&body))).is_none());
}

#[tokio::test]
async fn cross_site_reward_posts_are_rejected() {
    let app = TestApp::start().await;
    let id = reward_id(&app, "Sticker").await;
    for (path, form) in [
        (
            REWARD_EDIT_PATH,
            format!("item={id}&{}", reward_form("Hacked", "💀", "1")),
        ),
        (REWARD_NEW_PATH, reward_form("Hacked", "💀", "1")),
        (REWARD_RETIRE_PATH, format!("item={id}")),
        (REWARD_REORDER_PATH, format!("order={id}")),
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
    let sheet = spend(&app, "Kid 1").await;
    assert_eq!(
        tile(select(&sheet, "[data-reward-name=\"Sticker\"]")[0]),
        ("Sticker".to_string(), "⭐".to_string(), 5)
    );
    assert!(select(&sheet, "[data-reward-name=\"Hacked\"]").is_empty());
}
