mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{Fragment, HOST, TestApp};
use jiff::SignedDuration;
use pebblepot::web::{BOARD_REFRESH_PATH, REDEEM_PATH, SPEND_PATH};
use scraper::{Html, Selector};

/// The Spend sheet, read through its data-* markers.
struct SpendSheet(Html);

#[derive(Debug, PartialEq)]
struct Tile {
    name: String,
    price: i64,
    locked: bool,
    /// "N more to go" on a locked tile.
    more_to_go: Option<i64>,
}

impl SpendSheet {
    fn one(&self, selector: &str) -> Option<scraper::ElementRef<'_>> {
        self.0.select(&Selector::parse(selector).unwrap()).next()
    }

    /// The family member whose pebbles are being spent, if one has been picked.
    fn spending_for(&self) -> Option<String> {
        let sheet = self.one("[data-spend-sheet]").expect("no Spend sheet");
        let who = sheet.value().attr("data-spend-sheet").unwrap();
        (!who.is_empty()).then(|| who.to_string())
    }

    /// How many pebbles the sheet says can be spent.
    fn spendable(&self) -> i64 {
        self.one("[data-spendable]")
            .expect("no spendable amount")
            .value()
            .attr("data-spendable")
            .unwrap()
            .parse()
            .unwrap()
    }

    /// Names of the family members offered in the "whose pebbles?" picker.
    fn picker(&self) -> Vec<String> {
        self.0
            .select(&Selector::parse("[data-spend-for]").unwrap())
            .map(|e| e.value().attr("data-member-name").unwrap().to_string())
            .collect()
    }

    fn tiles(&self) -> Vec<Tile> {
        self.0
            .select(&Selector::parse("[data-reward]").unwrap())
            .map(|e| {
                let v = e.value();
                Tile {
                    name: v.attr("data-reward-name").unwrap().to_string(),
                    price: v.attr("data-price").unwrap().parse().unwrap(),
                    locked: match v.attr("data-state").unwrap() {
                        "locked" => true,
                        "unlocked" => false,
                        other => panic!("unexpected tile state {other}"),
                    },
                    more_to_go: v.attr("data-more-to-go").map(|n| n.parse().unwrap()),
                }
            })
            .collect()
    }

    fn tile(&self, name: &str) -> Tile {
        self.tiles()
            .into_iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("no {name} tile"))
    }

    /// Whether the tile can be tapped to redeem it.
    fn redeemable(&self, name: &str) -> bool {
        let tile = self
            .one(&format!("[data-reward-name=\"{name}\"]"))
            .unwrap_or_else(|| panic!("no {name} tile"));
        tile.value().attr("hx-post") == Some(REDEEM_PATH)
    }
}

trait SpendApp {
    async fn spend_sheet(&self, member: Option<&str>) -> SpendSheet;
    async fn redeem(&self, member: &str, reward: &str) -> Fragment;
    async fn reward_id(&self, reward: &str) -> String;
}

impl SpendApp for TestApp {
    /// Taps 🎁 Spend (no member yet) or picks whose pebbles to spend.
    async fn spend_sheet(&self, member: Option<&str>) -> SpendSheet {
        let path = match member {
            None => SPEND_PATH.to_string(),
            Some(name) => {
                let id = self.board().await.member_id(name);
                format!("{SPEND_PATH}?member={id}")
            }
        };
        let (status, body) = self.get(&path).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        SpendSheet(Html::parse_fragment(&body))
    }

    /// Taps a reward tile on `member`'s Spend sheet.
    async fn redeem(&self, member: &str, reward: &str) -> Fragment {
        let member = self.board().await.member_id(member);
        let item = self.reward_id(reward).await;
        let (status, body) = self
            .post_form(REDEEM_PATH, &format!("member={member}&item={item}"))
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        Fragment(Html::parse_fragment(&body))
    }

    async fn reward_id(&self, reward: &str) -> String {
        let sheet = self.spend_sheet(Some("Mom")).await;
        let tile = sheet
            .one(&format!("[data-reward-name=\"{reward}\"]"))
            .unwrap_or_else(|| panic!("no {reward} tile"));
        tile.value().attr("data-reward").unwrap().to_string()
    }
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

#[tokio::test]
async fn the_spend_button_opens_a_sheet_asking_whose_pebbles() {
    let app = TestApp::start().await;
    let board = app.board().await;
    let button = board
        .0
        .select(&Selector::parse("[data-pots] [data-spend]").unwrap())
        .next()
        .unwrap();
    assert_eq!(button.value().attr("hx-get"), Some(SPEND_PATH));
    assert_eq!(button.value().attr("hx-target"), Some("#sheet"));

    let sheet = app.spend_sheet(None).await;
    assert_eq!(sheet.spending_for(), None);
    assert_eq!(sheet.picker(), ["Kid 1", "Kid 2", "Mom", "Dad"]);
    assert!(
        sheet.tiles().is_empty(),
        "no rewards until someone is picked"
    );
}

#[tokio::test]
async fn the_spend_sheet_shows_every_reward_in_order_with_its_price() {
    let app = TestApp::start().await;
    let sheet = app.spend_sheet(Some("Kid 2")).await;

    assert_eq!(
        sheet.spending_for(),
        Some(app.board().await.member_id("Kid 2"))
    );
    assert_eq!(sheet.spendable(), 0);
    assert_eq!(sheet.picker(), ["Kid 1", "Kid 2", "Mom", "Dad"]);
    let tiles: Vec<(String, i64)> = sheet
        .tiles()
        .into_iter()
        .map(|t| (t.name, t.price))
        .collect();
    let expected = [
        ("Sticker", 5),
        ("Extra bedtime story", 8),
        ("Pick dessert", 10),
        ("15 minutes extra screen time", 10),
        ("Choose dinner", 15),
        ("Stay up 15 minutes later", 15),
        ("Trip to the park", 15),
        ("Bake something together", 20),
        ("Pick the family movie", 20),
        ("Movie night with popcorn", 30),
        ("Playdate with a friend", 40),
        ("Small toy", 40),
        ("Day out", 60),
        ("Big toy", 100),
        ("Breakfast in bed (grown-ups)", 20),
        ("Sleep in (grown-ups)", 30),
        ("30 minutes of quiet (grown-ups)", 15),
    ]
    .map(|(name, price)| (name.to_string(), price));
    assert_eq!(tiles, expected);
}

#[tokio::test]
async fn rewards_beyond_the_pot_are_locked_with_how_many_more_to_go() {
    let app = kid_1_with_six_pebbles().await;
    let sheet = app.spend_sheet(Some("Kid 1")).await;

    assert_eq!(sheet.spendable(), 6);
    assert_eq!(
        sheet.tile("Sticker"),
        Tile {
            name: "Sticker".into(),
            price: 5,
            locked: false,
            more_to_go: None
        }
    );
    assert!(sheet.redeemable("Sticker"));
    assert_eq!(
        sheet.tile("Extra bedtime story"),
        Tile {
            name: "Extra bedtime story".into(),
            price: 8,
            locked: true,
            more_to_go: Some(2)
        }
    );
    assert!(!sheet.redeemable("Extra bedtime story"));
    assert_eq!(sheet.tile("Big toy").more_to_go, Some(94));
    assert!(sheet.0.html().contains("🔒 2 more to go"));

    // Someone else's pebbles don't unlock anything for Kid 1, and vice versa.
    let kid_2 = app.spend_sheet(Some("Kid 2")).await;
    assert!(kid_2.tiles().iter().all(|t| t.locked));
    assert_eq!(kid_2.tile("Sticker").more_to_go, Some(5));
}

#[tokio::test]
async fn a_reward_priced_exactly_at_the_pot_is_unlocked() {
    let app = TestApp::start().await;
    app.earn_chore("Dad", "Rake leaves").await;
    app.earn_chore("Dad", "Make bed").await;

    let sheet = app.spend_sheet(Some("Dad")).await;
    assert!(!sheet.tile("Sticker").locked);
    assert!(sheet.tile("Extra bedtime story").locked);
}

#[tokio::test]
async fn redeeming_an_affordable_reward_takes_its_price_from_the_pot() {
    let app = kid_1_with_six_pebbles().await;

    let reply = app.redeem("Kid 1", "Sticker").await;

    assert_eq!(reply.toast().kind, "redeemed");
    assert_eq!(reply.toast().text, "🦊 ⭐ -5");
    assert_eq!(app.pot("Kid 1").await, 1);
    assert_eq!(app.pot("Kid 2").await, 0);
    let list = app.today_list("Kid 1").await;
    let top = &list.entries()[0];
    assert_eq!((top.name.as_str(), top.amount), ("Sticker", -5));
}

#[tokio::test]
async fn the_redeem_reply_redraws_the_pots_and_the_spend_sheet() {
    let app = kid_1_with_six_pebbles().await;

    let reply = app.redeem("Kid 1", "Sticker").await;

    assert!(reply.pots().contains(&common::Pot {
        name: "Kid 1".into(),
        pebbles: 1
    }));
    let sheet = SpendSheet(reply.0.clone());
    assert_eq!(sheet.spendable(), 1);
    assert_eq!(sheet.tile("Sticker").more_to_go, Some(4));
    // Swapped in out of band: the tap came from inside the sheet.
    let html = reply.0.html();
    assert!(html.contains(r#"id="spend-"#) && html.contains("hx-swap-oob"));
}

#[tokio::test]
async fn the_redeem_entry_keeps_its_own_copy_of_the_rewards_name_and_picture() {
    let app = kid_1_with_six_pebbles().await;
    app.redeem("Kid 1", "Sticker").await;

    let list = app.today_list("Kid 1").await;
    let html = list.0.html();
    let entry = list.entries().remove(0);
    assert_eq!(entry.name, "Sticker");
    assert_eq!(entry.amount, -5);
    assert!(html.contains("⭐"));
}

#[tokio::test]
async fn a_reward_priced_above_the_pot_is_refused_and_the_pot_is_unchanged() {
    let app = kid_1_with_six_pebbles().await;

    let reply = app.redeem("Kid 1", "Extra bedtime story").await;

    assert_eq!(reply.toast().kind, "refused");
    assert_eq!(reply.toast().refusal.as_deref(), Some("not_enough_pebbles"));
    assert_eq!(reply.toast_undo_entry(), None);
    assert_eq!(app.pot("Kid 1").await, 6);
    assert_eq!(
        app.today_list("Kid 1").await.names(),
        ["Help cook", "Tidy toys", "Make bed"]
    );
}

#[tokio::test]
async fn spending_down_to_zero_is_allowed_but_never_below() {
    let app = TestApp::start().await;
    app.earn_chore("Kid 2", "Make bed").await;
    app.earn_chore("Kid 2", "Rake leaves").await; // 1 + 4 = 5

    let reply = app.redeem("Kid 2", "Sticker").await;
    assert_eq!(reply.toast().kind, "redeemed");
    assert_eq!(app.pot("Kid 2").await, 0);

    let again = app.redeem("Kid 2", "Sticker").await;
    assert_eq!(again.toast().refusal.as_deref(), Some("not_enough_pebbles"));
    assert_eq!(app.pot("Kid 2").await, 0);
}

#[tokio::test]
async fn a_pot_left_below_zero_by_an_undo_cannot_spend() {
    // Undo may leave a pot below zero (spec, Further Notes); redeeming must not dig deeper.
    let app = TestApp::start().await;
    let rake = app.earn_chore("Kid 2", "Rake leaves").await; // 4
    app.earn_chore("Kid 2", "Make bed").await; // 5
    app.redeem("Kid 2", "Sticker").await; // 0
    app.undo(&rake.toast_undo_entry().unwrap()).await;
    assert_eq!(app.pot("Kid 2").await, -4);

    let refused = app.redeem("Kid 2", "Sticker").await;
    assert_eq!(
        refused.toast().refusal.as_deref(),
        Some("not_enough_pebbles")
    );
    assert_eq!(app.pot("Kid 2").await, -4);
    let sheet = app.spend_sheet(Some("Kid 2")).await;
    assert_eq!(sheet.tile("Sticker").more_to_go, Some(9));
}

#[tokio::test]
async fn the_redeem_toast_offers_undo_which_puts_the_pebbles_back() {
    let app = kid_1_with_six_pebbles().await;
    let reply = app.redeem("Kid 1", "Sticker").await;
    let entry = reply.toast_undo_entry().expect("redeem toast offers undo");

    let undone = app.undo(&entry).await;

    assert_eq!(undone.toast().kind, "undone");
    assert_eq!(undone.toast().text, "🦊 ⭐ undone");
    assert_eq!(app.pot("Kid 1").await, 6);
    assert!(
        !app.today_list("Kid 1")
            .await
            .names()
            .contains(&"Sticker".to_string())
    );
    assert!(!app.spend_sheet(Some("Kid 1")).await.tile("Sticker").locked);
}

#[tokio::test]
async fn a_redeem_can_be_undone_from_the_today_list() {
    let app = kid_1_with_six_pebbles().await;
    app.redeem("Kid 1", "Sticker").await;
    let entry = app.today_list("Kid 1").await.entry("Sticker");

    let reply = app.undo(&entry).await;

    let list = reply.today_list().expect("reply redraws the today list");
    assert_eq!(list.names(), ["Help cook", "Tidy toys", "Make bed"]);
    assert_eq!(app.pot("Kid 1").await, 6);
}

#[tokio::test]
async fn yesterdays_redeem_cannot_be_undone() {
    let app = kid_1_with_six_pebbles().await;
    let entry = app
        .redeem("Kid 1", "Sticker")
        .await
        .toast_undo_entry()
        .unwrap();

    app.clock.advance(SignedDuration::from_hours(24));
    let refused = app.undo(&entry).await;

    assert_eq!(refused.toast().refusal.as_deref(), Some("not_undoable"));
    assert_eq!(app.pot("Kid 1").await, 1);
}

#[tokio::test]
async fn only_rewards_can_be_redeemed() {
    let app = kid_1_with_six_pebbles().await;
    let board = app.board().await;
    let (member, chore) = (board.member_id("Kid 1"), board.item_id("Make bed"));

    let (status, body) = app
        .post_form(REDEEM_PATH, &format!("member={member}&item={chore}"))
        .await;

    assert_eq!(status, StatusCode::OK);
    let reply = Fragment(Html::parse_fragment(&body));
    assert_eq!(reply.toast().refusal.as_deref(), Some("unknown_item"));
    assert_eq!(app.pot("Kid 1").await, 6);
}

#[tokio::test]
async fn spending_for_someone_not_on_the_board_is_refused() {
    let app = TestApp::start().await;
    let (status, _) = app.get(&format!("{SPEND_PATH}?member=999")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let item = app.reward_id("Sticker").await;
    let (status, body) = app
        .post_form(REDEEM_PATH, &format!("member=999&item={item}"))
        .await;
    assert_eq!(status, StatusCode::OK);
    let reply = Fragment(Html::parse_fragment(&body));
    assert_eq!(reply.toast().refusal.as_deref(), Some("unknown_member"));
}

#[tokio::test]
async fn an_open_spend_sheet_is_redrawn_by_the_live_refresh() {
    let app = kid_1_with_six_pebbles().await;
    let member = app.board().await.member_id("Kid 1");
    // Another device redeems while this one has Kid 1's Spend sheet open.
    app.redeem("Kid 1", "Sticker").await;

    let (status, body) = app
        .get(&format!("{BOARD_REFRESH_PATH}?q=&spend={member}"))
        .await;

    assert_eq!(status, StatusCode::OK);
    let sheet = SpendSheet(Html::parse_fragment(&body));
    assert_eq!(sheet.spendable(), 1);
    assert!(sheet.tile("Sticker").locked);
    assert!(body.contains(&format!(r#"id="spend-{member}""#)));

    // Without an open sheet the refresh carries none.
    let (_, body) = app.get(&format!("{BOARD_REFRESH_PATH}?q=")).await;
    assert!(!body.contains("data-spend-sheet"));
}

#[tokio::test]
async fn cross_site_redeem_posts_are_rejected() {
    let app = kid_1_with_six_pebbles().await;
    let member = app.board().await.member_id("Kid 1");
    let item = app.reward_id("Sticker").await;
    let request = Request::post(REDEEM_PATH)
        .header(header::HOST, HOST)
        .header(header::ORIGIN, "http://evil.example")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(Body::from(format!("member={member}&item={item}")))
        .unwrap();
    let (status, _) = app.send(request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(app.pot("Kid 1").await, 6);
}
