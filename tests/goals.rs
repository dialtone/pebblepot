mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{Fragment, HOST, TestApp};
use jiff::SignedDuration;
use pebblepot::web::{
    BOARD_REFRESH_PATH, EARN_GOOD_DEED_PATH, GOAL_GIVE_UP_PATH, GOAL_GOT_IT_PATH, GOAL_SET_PATH,
    REDEEM_PATH, SPEND_PATH,
};
use scraper::{ElementRef, Html, Selector};

fn one<'a>(html: &'a Html, selector: &str) -> Option<ElementRef<'a>> {
    html.select(&Selector::parse(selector).unwrap()).next()
}

fn attr(el: ElementRef<'_>, name: &str) -> String {
    el.value()
        .attr(name)
        .unwrap_or_else(|| panic!("no {name} on {}", el.html()))
        .to_string()
}

/// One family member's pot card, read through its data-* markers.
#[derive(Debug, PartialEq)]
struct PotCard {
    pot: i64,
    /// "pebbles", or "goal" while saving for one.
    shows: String,
    goal: Option<GoalCard>,
}

#[derive(Debug, PartialEq)]
struct GoalCard {
    name: String,
    /// "saving" or "ready".
    state: String,
    saved: i64,
    to_go: i64,
    /// How much of the goal picture is in colour, in percent.
    fill: i64,
    got_it: bool,
}

fn pot_card(html: &Html, member_id: &str) -> PotCard {
    let card = one(html, &format!("[data-pots] [data-member=\"{member_id}\"]"))
        .unwrap_or_else(|| panic!("no pot card for {member_id}"));
    let goal = card.value().attr("data-goal").map(|name| GoalCard {
        name: name.to_string(),
        state: attr(card, "data-goal-state"),
        saved: attr(card, "data-saved").parse().unwrap(),
        to_go: attr(card, "data-to-go").parse().unwrap(),
        fill: attr(card, "data-fill").parse().unwrap(),
        got_it: card
            .select(&Selector::parse("[data-got-it]").unwrap())
            .next()
            .is_some_and(|b| b.value().attr("hx-post") == Some(GOAL_GOT_IT_PATH)),
    });
    PotCard {
        pot: attr(card, "data-pot").parse().unwrap(),
        shows: attr(card, "data-shows"),
        goal,
    }
}

/// The Spend sheet, read through its data-* markers.
struct SpendSheet(Html);

impl SpendSheet {
    fn spendable(&self) -> i64 {
        attr(one(&self.0, "[data-spendable]").unwrap(), "data-spendable")
            .parse()
            .unwrap()
    }

    fn tile(&self, reward: &str) -> ElementRef<'_> {
        one(&self.0, &format!("[data-reward-name=\"{reward}\"]"))
            .unwrap_or_else(|| panic!("no {reward} tile"))
    }

    fn locked(&self, reward: &str) -> bool {
        attr(self.tile(reward), "data-state") == "locked"
    }

    fn more_to_go(&self, reward: &str) -> Option<i64> {
        self.tile(reward)
            .value()
            .attr("data-more-to-go")
            .map(|n| n.parse().unwrap())
    }

    /// Whether the tile offers ⭐ Save for this.
    fn offers_save(&self, reward: &str) -> bool {
        self.tile(reward)
            .select(&Selector::parse("[data-save-for]").unwrap())
            .next()
            .is_some_and(|b| b.value().attr("hx-post") == Some(GOAL_SET_PATH))
    }

    fn save_offers(&self) -> usize {
        self.0
            .select(&Selector::parse("[data-save-for]").unwrap())
            .count()
    }

    /// The goal shown at the top of the sheet: (name, state).
    fn goal(&self) -> Option<(String, String)> {
        one(&self.0, "[data-spend-goal]")
            .map(|g| (attr(g, "data-spend-goal"), attr(g, "data-goal-state")))
    }

    fn offers(&self, marker: &str, path: &str) -> bool {
        one(&self.0, &format!("[{marker}]"))
            .is_some_and(|b| b.value().attr("hx-post") == Some(path))
    }
}

trait GoalApp {
    async fn earn(&self, member: &str, pebbles: i64);
    async fn spend_sheet(&self, member: &str) -> SpendSheet;
    async fn reward_id(&self, reward: &str) -> String;
    async fn post_ok(&self, path: &str, form: &str) -> Fragment;
    async fn save_for(&self, member: &str, reward: &str) -> Fragment;
    async fn redeem(&self, member: &str, reward: &str) -> Fragment;
    async fn give_up(&self, member: &str) -> Fragment;
    async fn got_it(&self, member: &str) -> Fragment;
    async fn card(&self, member: &str) -> PotCard;
}

impl GoalApp for TestApp {
    /// Earns exactly `pebbles` with good deeds (Helped without being asked = 3, Kind words = 1),
    /// a few seconds apart so none is a repeat tap.
    async fn earn(&self, member: &str, mut pebbles: i64) {
        let board = self.board().await;
        let id = board.member_id(member);
        let (three, one) = (
            board.item_id("Helped without being asked"),
            board.item_id("Kind words"),
        );
        while pebbles > 0 {
            let (item, value) = if pebbles >= 3 { (&three, 3) } else { (&one, 1) };
            self.clock.advance(SignedDuration::from_secs(3));
            let (status, body) = self
                .post_form(EARN_GOOD_DEED_PATH, &format!("member={id}&item={item}"))
                .await;
            assert_eq!(status, StatusCode::OK, "{body}");
            assert!(!body.contains("data-refusal"), "{body}");
            pebbles -= value;
        }
    }

    async fn spend_sheet(&self, member: &str) -> SpendSheet {
        let id = self.board().await.member_id(member);
        let (status, body) = self.get(&format!("{SPEND_PATH}?member={id}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        SpendSheet(Html::parse_fragment(&body))
    }

    async fn reward_id(&self, reward: &str) -> String {
        attr(self.spend_sheet("Mom").await.tile(reward), "data-reward")
    }

    async fn post_ok(&self, path: &str, form: &str) -> Fragment {
        let (status, body) = self.post_form(path, form).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        Fragment(Html::parse_fragment(&body))
    }

    /// Taps ⭐ Save for this on a reward tile in `member`'s Spend sheet.
    async fn save_for(&self, member: &str, reward: &str) -> Fragment {
        let member = self.board().await.member_id(member);
        let item = self.reward_id(reward).await;
        self.post_ok(GOAL_SET_PATH, &format!("member={member}&item={item}"))
            .await
    }

    async fn redeem(&self, member: &str, reward: &str) -> Fragment {
        let member = self.board().await.member_id(member);
        let item = self.reward_id(reward).await;
        self.post_ok(REDEEM_PATH, &format!("member={member}&item={item}"))
            .await
    }

    async fn give_up(&self, member: &str) -> Fragment {
        let member = self.board().await.member_id(member);
        self.post_ok(GOAL_GIVE_UP_PATH, &format!("member={member}"))
            .await
    }

    async fn got_it(&self, member: &str) -> Fragment {
        let member = self.board().await.member_id(member);
        self.post_ok(GOAL_GOT_IT_PATH, &format!("member={member}"))
            .await
    }

    async fn card(&self, member: &str) -> PotCard {
        let board = self.board().await;
        pot_card(&board.0, &board.member_id(member))
    }
}

/// The spec's example: Kid 1 has 8 pebbles.
async fn kid_1_with_eight() -> TestApp {
    let app = TestApp::start().await;
    app.earn("Kid 1", 8).await;
    assert_eq!(app.pot("Kid 1").await, 8);
    app
}

/// Kid 1 had 8 and chose Movie night with popcorn (30): pot −22, saved 8.
async fn kid_1_saving_for_movie_night() -> TestApp {
    let app = kid_1_with_eight().await;
    let reply = app.save_for("Kid 1", "Movie night with popcorn").await;
    assert_eq!(reply.toast().kind, "goal_set", "{}", reply.0.html());
    app
}

fn refusal(reply: &Fragment) -> Option<String> {
    reply.toast().refusal
}

#[tokio::test]
async fn save_for_this_is_offered_only_on_rewards_priced_above_the_pot() {
    let app = kid_1_with_eight().await;
    let sheet = app.spend_sheet("Kid 1").await;

    assert!(!sheet.locked("Sticker"));
    assert!(!sheet.offers_save("Sticker"));
    assert!(
        !sheet.locked("Extra bedtime story"),
        "price = pot is affordable"
    );
    assert!(!sheet.offers_save("Extra bedtime story"));
    assert!(sheet.locked("Pick dessert"));
    assert!(sheet.offers_save("Pick dessert"));
    assert!(sheet.offers_save("Movie night with popcorn"));
    assert!(sheet.offers_save("Big toy"));
    assert!(sheet.0.html().contains("⭐ Save for this"));
    // Still locked with how far there is to go.
    assert_eq!(sheet.more_to_go("Movie night with popcorn"), Some(22));
}

#[tokio::test]
async fn setting_a_goal_redeems_it_now_and_takes_the_pot_below_zero_by_the_shortfall() {
    let app = kid_1_with_eight().await;

    let reply = app.save_for("Kid 1", "Movie night with popcorn").await;

    assert_eq!(reply.toast().kind, "goal_set");
    assert!(reply.toast().text.contains("🍿"), "{:?}", reply.toast());
    assert!(reply.toast_undo_entry().is_some());
    assert!(reply.pots().contains(&common::Pot {
        name: "Kid 1".into(),
        pebbles: -22
    }));
    assert_eq!(app.pot("Kid 1").await, -22);
    assert_eq!(app.pot("Kid 2").await, 0);
    let top = app.today_list("Kid 1").await.entries().remove(0);
    assert_eq!(
        (top.name.as_str(), top.amount),
        ("Movie night with popcorn", -30)
    );
}

#[tokio::test]
async fn the_pot_card_shows_the_goal_picture_filling_with_n_to_go() {
    let app = kid_1_saving_for_movie_night().await;

    let card = app.card("Kid 1").await;
    assert_eq!(
        card,
        PotCard {
            pot: -22,
            shows: "goal".into(),
            goal: Some(GoalCard {
                name: "Movie night with popcorn".into(),
                state: "saving".into(),
                saved: 8,
                to_go: 22,
                fill: 26, // 8 / 30, rounded down
                got_it: false,
            }),
        }
    );
    let html = app.board().await.0.html();
    assert!(html.contains("22 to go"));
    assert!(html.contains("🍿"));
    // Everyone else still shows pebbles.
    assert_eq!(app.card("Kid 2").await.shows, "pebbles");
    assert_eq!(app.card("Kid 2").await.goal, None);

    // Saving more fills more of the picture.
    app.earn("Kid 1", 7).await;
    let goal = app.card("Kid 1").await.goal.unwrap();
    assert_eq!((goal.saved, goal.to_go, goal.fill), (15, 15, 50));
}

#[tokio::test]
async fn a_pot_card_with_a_goal_still_opens_the_today_list() {
    let app = kid_1_saving_for_movie_night().await;
    let board = app.board().await;
    let id = board.member_id("Kid 1");
    let card = one(&board.0, &format!("[data-pots] [data-member=\"{id}\"]")).unwrap();
    let opener = card
        .select(&Selector::parse("[hx-get]").unwrap())
        .next()
        .expect("the card opens the today list");
    assert_eq!(
        opener.value().attr("hx-get"),
        Some(format!("{}?member={id}", pebblepot::web::TODAY_PATH).as_str())
    );
}

#[tokio::test]
async fn only_one_goal_at_a_time() {
    let app = kid_1_saving_for_movie_night().await;

    let sheet = app.spend_sheet("Kid 1").await;
    assert_eq!(sheet.save_offers(), 0, "no Save for this during a goal");

    let refused = app.save_for("Kid 1", "Big toy").await;
    assert_eq!(refusal(&refused).as_deref(), Some("goal_already_active"));
    assert_eq!(refused.toast_undo_entry(), None);
    assert_eq!(app.pot("Kid 1").await, -22);
    assert_eq!(
        app.card("Kid 1").await.goal.unwrap().name,
        "Movie night with popcorn"
    );

    // Another family member can still set their own.
    let kid_2 = app.spend_sheet("Kid 2").await;
    assert!(kid_2.offers_save("Sticker"));
    app.save_for("Kid 2", "Sticker").await;
    assert_eq!(app.pot("Kid 2").await, -5);
}

#[tokio::test]
async fn a_goal_cannot_be_set_for_a_reward_that_is_already_affordable() {
    let app = kid_1_with_eight().await;

    for reward in ["Sticker", "Extra bedtime story"] {
        let refused = app.save_for("Kid 1", reward).await;
        assert_eq!(refusal(&refused).as_deref(), Some("affordable"), "{reward}");
    }
    assert_eq!(app.pot("Kid 1").await, 8);
    assert_eq!(app.card("Kid 1").await.goal, None);
}

#[tokio::test]
async fn while_saving_spend_shows_the_saved_pebbles_as_n_to_spend() {
    let app = kid_1_saving_for_movie_night().await;

    let sheet = app.spend_sheet("Kid 1").await;
    assert_eq!(sheet.spendable(), 8);
    assert!(sheet.0.html().contains("<b>8</b> to spend"));
    assert!(!sheet.locked("Sticker"));
    assert!(!sheet.locked("Extra bedtime story"));
    assert!(sheet.locked("Pick dessert"));
    assert_eq!(sheet.more_to_go("Pick dessert"), Some(2));
    assert_eq!(
        sheet.goal(),
        Some(("Movie night with popcorn".into(), "saving".into()))
    );
}

#[tokio::test]
async fn spending_while_saving_is_allowed_only_from_what_is_saved() {
    // The spec's example: pot 8, goal 30 → −22, saved 8; Sticker 5 allowed, then Dessert 10 refused.
    let app = kid_1_saving_for_movie_night().await;

    let sticker = app.redeem("Kid 1", "Sticker").await;
    assert_eq!(sticker.toast().kind, "redeemed");
    assert_eq!(app.pot("Kid 1").await, -27);
    let sheet = SpendSheet(sticker.0.clone());
    assert_eq!(sheet.spendable(), 3, "the redrawn sheet shows what's left");
    assert_eq!(app.card("Kid 1").await.goal.unwrap().saved, 3);

    let dessert = app.redeem("Kid 1", "Pick dessert").await;
    assert_eq!(refusal(&dessert).as_deref(), Some("not_enough_pebbles"));
    assert_eq!(app.pot("Kid 1").await, -27);
    let sheet = app.spend_sheet("Kid 1").await;
    assert_eq!(sheet.spendable(), 3);
    assert_eq!(sheet.more_to_go("Pick dessert"), Some(7));
    assert_eq!(sheet.more_to_go("Sticker"), Some(2));
}

#[tokio::test]
async fn saved_can_be_spent_down_to_exactly_zero_but_not_below() {
    let app = kid_1_saving_for_movie_night().await;

    let story = app.redeem("Kid 1", "Extra bedtime story").await; // 8 = all of saved
    assert_eq!(story.toast().kind, "redeemed");
    assert_eq!(app.pot("Kid 1").await, -30);
    assert_eq!(app.card("Kid 1").await.goal.unwrap().fill, 0);

    let sticker = app.redeem("Kid 1", "Sticker").await;
    assert_eq!(refusal(&sticker).as_deref(), Some("not_enough_pebbles"));
    assert_eq!(app.pot("Kid 1").await, -30);
}

#[tokio::test]
async fn the_goal_is_ready_once_the_pot_is_back_to_zero() {
    let app = kid_1_saving_for_movie_night().await;

    app.earn("Kid 1", 21).await;
    let almost = app.card("Kid 1").await.goal.unwrap();
    assert_eq!((almost.state.as_str(), almost.to_go), ("saving", 1));
    assert!(!almost.got_it);

    app.earn("Kid 1", 1).await;
    let card = app.card("Kid 1").await;
    assert_eq!(card.pot, 0);
    assert_eq!(
        card.goal,
        Some(GoalCard {
            name: "Movie night with popcorn".into(),
            state: "ready".into(),
            saved: 30,
            to_go: 0,
            fill: 100,
            got_it: true,
        })
    );
    assert!(app.board().await.0.html().contains("🎉 Ready!"));

    // Past zero it stays Ready, fully coloured.
    app.earn("Kid 1", 3).await;
    let goal = app.card("Kid 1").await.goal.unwrap();
    assert_eq!(
        (goal.state.as_str(), goal.fill, goal.saved),
        ("ready", 100, 33)
    );
    let sheet = app.spend_sheet("Kid 1").await;
    assert_eq!(
        sheet.goal(),
        Some(("Movie night with popcorn".into(), "ready".into()))
    );
    assert!(sheet.offers("data-got-it", GOAL_GOT_IT_PATH));
}

#[tokio::test]
async fn got_it_hands_the_goal_over_and_the_card_shows_pebbles_again() {
    let app = kid_1_saving_for_movie_night().await;
    app.earn("Kid 1", 25).await; // pot 3
    let before = app.today_list("Kid 1").await.entries();

    let reply = app.got_it("Kid 1").await;

    assert_eq!(reply.toast().kind, "handed_over");
    assert_eq!(reply.toast_undo_entry(), None);
    let card = app.card("Kid 1").await;
    assert_eq!(
        card,
        PotCard {
            pot: 3,
            shows: "pebbles".into(),
            goal: None
        }
    );
    assert_eq!(
        pot_card(&reply.0, &app.board().await.member_id("Kid 1")).goal,
        None
    );
    // Handing over writes nothing to the log.
    assert_eq!(app.today_list("Kid 1").await.entries(), before);
    // And a new goal can be set.
    let sheet = app.spend_sheet("Kid 1").await;
    assert_eq!(sheet.goal(), None);
    assert!(sheet.offers_save("Sticker"));
    assert_eq!(sheet.spendable(), 3);
}

#[tokio::test]
async fn got_it_is_refused_until_the_goal_is_ready() {
    let app = kid_1_saving_for_movie_night().await;

    let refused = app.got_it("Kid 1").await;
    assert_eq!(refusal(&refused).as_deref(), Some("goal_not_ready"));
    assert_eq!(app.card("Kid 1").await.goal.unwrap().state, "saving");

    let nobody = app.got_it("Kid 2").await;
    assert_eq!(refusal(&nobody).as_deref(), Some("no_goal"));
}

#[tokio::test]
async fn giving_up_a_goal_writes_a_refund_and_restores_the_pot() {
    let app = kid_1_saving_for_movie_night().await;
    let sheet = app.spend_sheet("Kid 1").await;
    assert!(sheet.offers("data-give-up", GOAL_GIVE_UP_PATH));

    let reply = app.give_up("Kid 1").await;

    assert_eq!(reply.toast().kind, "goal_given_up");
    assert!(reply.toast_undo_entry().is_some());
    assert_eq!(app.pot("Kid 1").await, 8);
    assert_eq!(
        app.card("Kid 1").await,
        PotCard {
            pot: 8,
            shows: "pebbles".into(),
            goal: None
        }
    );
    // A new refund entry; the goal-set entry stays in the log.
    let entries = app.today_list("Kid 1").await.entries();
    let (refund, set) = (&entries[0], &entries[1]);
    assert_eq!(
        (refund.name.as_str(), refund.amount),
        ("Movie night with popcorn", 30)
    );
    assert_eq!(
        (set.name.as_str(), set.amount),
        ("Movie night with popcorn", -30)
    );
    // Saving can start again.
    assert!(app.spend_sheet("Kid 1").await.offers_save("Pick dessert"));
    assert_eq!(
        refusal(&app.give_up("Kid 1").await).as_deref(),
        Some("no_goal")
    );
}

#[tokio::test]
async fn giving_up_after_spending_from_saved_refunds_only_the_goal_price() {
    let app = kid_1_saving_for_movie_night().await;
    app.redeem("Kid 1", "Sticker").await; // −27

    app.give_up("Kid 1").await;

    assert_eq!(app.pot("Kid 1").await, 3);
}

#[tokio::test]
async fn undoing_the_goal_set_entry_on_the_same_day_removes_the_goal() {
    let app = kid_1_with_eight().await;
    let set = app.save_for("Kid 1", "Movie night with popcorn").await;
    let entry = set.toast_undo_entry().unwrap();

    let undone = app.undo(&entry).await;

    assert_eq!(undone.toast().kind, "undone");
    assert_eq!(app.pot("Kid 1").await, 8);
    assert_eq!(
        pot_card(&undone.0, &app.board().await.member_id("Kid 1")).shows,
        "pebbles"
    );
    assert_eq!(app.card("Kid 1").await.goal, None);
    assert!(
        !app.today_list("Kid 1")
            .await
            .names()
            .contains(&"Movie night with popcorn".to_string())
    );
    // Gone entirely: a new goal can be set, and there is nothing to give up.
    assert_eq!(
        refusal(&app.give_up("Kid 1").await).as_deref(),
        Some("no_goal")
    );
    let sheet = app.spend_sheet("Kid 1").await;
    assert_eq!(sheet.goal(), None);
    assert!(sheet.offers_save("Big toy"));
}

#[tokio::test]
async fn the_goal_set_entry_can_be_undone_from_the_today_list() {
    let app = kid_1_saving_for_movie_night().await;
    let entry = app
        .today_list("Kid 1")
        .await
        .entry("Movie night with popcorn");

    app.undo(&entry).await;

    assert_eq!(app.pot("Kid 1").await, 8);
    assert_eq!(app.card("Kid 1").await.goal, None);
}

#[tokio::test]
async fn yesterdays_goal_cannot_be_undone() {
    let app = kid_1_with_eight().await;
    let entry = app
        .save_for("Kid 1", "Movie night with popcorn")
        .await
        .toast_undo_entry()
        .unwrap();

    app.clock.advance(SignedDuration::from_hours(24));
    let refused = app.undo(&entry).await;

    assert_eq!(refusal(&refused).as_deref(), Some("not_undoable"));
    assert_eq!(app.pot("Kid 1").await, -22);
    assert_eq!(app.card("Kid 1").await.goal.unwrap().state, "saving");
}

#[tokio::test]
async fn undoing_a_given_up_goal_set_entry_also_removes_its_refund() {
    // Otherwise set, give up, undo the set would leave the price as free pebbles.
    let app = kid_1_saving_for_movie_night().await;
    let set = app
        .today_list("Kid 1")
        .await
        .entry("Movie night with popcorn");
    app.give_up("Kid 1").await;

    app.undo(&set).await;

    assert_eq!(app.pot("Kid 1").await, 8);
    assert!(
        !app.today_list("Kid 1")
            .await
            .names()
            .contains(&"Movie night with popcorn".to_string())
    );
}

#[tokio::test]
async fn undoing_the_refund_takes_the_goal_back_up() {
    let app = kid_1_saving_for_movie_night().await;
    let refund = app.give_up("Kid 1").await.toast_undo_entry().unwrap();

    app.undo(&refund).await;

    assert_eq!(app.pot("Kid 1").await, -22);
    let goal = app.card("Kid 1").await.goal.expect("saving again");
    assert_eq!((goal.state.as_str(), goal.saved), ("saving", 8));
}

#[tokio::test]
async fn mom_and_dad_can_set_goals_the_same_way() {
    let app = TestApp::start().await;
    app.earn("Dad", 12).await;
    assert!(
        app.spend_sheet("Dad")
            .await
            .offers_save("Sleep in (grown-ups)")
    );

    app.save_for("Dad", "Sleep in (grown-ups)").await;
    assert_eq!(app.pot("Dad").await, -18);
    let goal = app.card("Dad").await.goal.unwrap();
    assert_eq!(
        (goal.name.as_str(), goal.to_go),
        ("Sleep in (grown-ups)", 18)
    );

    app.save_for("Mom", "30 minutes of quiet (grown-ups)").await;
    assert_eq!(app.pot("Mom").await, -15);
    app.earn("Mom", 15).await;
    app.got_it("Mom").await;
    assert_eq!(app.card("Mom").await.shows, "pebbles");
}

#[tokio::test]
async fn the_live_refresh_shows_the_goal_on_every_board() {
    let app = kid_1_saving_for_movie_night().await;
    let member = app.board().await.member_id("Kid 1");

    let (status, body) = app
        .get(&format!("{BOARD_REFRESH_PATH}?q=&spend={member}"))
        .await;

    assert_eq!(status, StatusCode::OK);
    let html = Html::parse_fragment(&body);
    assert_eq!(pot_card(&html, &member).goal.unwrap().to_go, 22);
    assert_eq!(SpendSheet(html).spendable(), 8);
}

#[tokio::test]
async fn goal_commands_for_unknown_people_or_items_are_refused() {
    let app = kid_1_with_eight().await;
    let board = app.board().await;
    let (kid_1, chore) = (board.member_id("Kid 1"), board.item_id("Make bed"));
    let reward = app.reward_id("Big toy").await;

    let not_a_reward = app
        .post_ok(GOAL_SET_PATH, &format!("member={kid_1}&item={chore}"))
        .await;
    assert_eq!(refusal(&not_a_reward).as_deref(), Some("unknown_item"));
    let nobody = app
        .post_ok(GOAL_SET_PATH, &format!("member=999&item={reward}"))
        .await;
    assert_eq!(refusal(&nobody).as_deref(), Some("unknown_member"));
    for path in [GOAL_GIVE_UP_PATH, GOAL_GOT_IT_PATH] {
        let nobody = app.post_ok(path, "member=999").await;
        assert_eq!(
            refusal(&nobody).as_deref(),
            Some("unknown_member"),
            "{path}"
        );
    }
    assert_eq!(app.pot("Kid 1").await, 8);
}

#[tokio::test]
async fn cross_site_goal_posts_are_rejected() {
    let app = kid_1_with_eight().await;
    let member = app.board().await.member_id("Kid 1");
    let item = app.reward_id("Big toy").await;
    for (path, form) in [
        (GOAL_SET_PATH, format!("member={member}&item={item}")),
        (GOAL_GIVE_UP_PATH, format!("member={member}")),
        (GOAL_GOT_IT_PATH, format!("member={member}")),
    ] {
        let request = Request::post(path)
            .header(header::HOST, HOST)
            .header(header::ORIGIN, "http://evil.example")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form))
            .unwrap();
        let (status, _) = app.send(request).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
    assert_eq!(app.pot("Kid 1").await, 8);
}

#[tokio::test]
async fn a_goal_survives_a_restart() {
    let mut app = kid_1_saving_for_movie_night().await;
    app.restart().await;
    assert_eq!(app.card("Kid 1").await.goal.unwrap().to_go, 22);
}
