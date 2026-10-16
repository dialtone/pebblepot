-- Goals: a reward redeemed before its owner can afford it. The goal keeps its own copy of the
-- reward's name, picture and price, like a log entry. "Ready" is not stored: it is an active goal
-- whose owner's pot is back to zero or more.
CREATE TABLE goal (
    id              INTEGER PRIMARY KEY,
    member_id       INTEGER NOT NULL REFERENCES family_member (id),
    -- The goal's own "goal set" log entry of -price. Undoing it deletes the goal.
    set_entry_id    INTEGER NOT NULL UNIQUE REFERENCES log_entry (id),
    -- The "goal refund" log entry of +price written when the goal was given up.
    refund_entry_id INTEGER UNIQUE REFERENCES log_entry (id),
    item_id         INTEGER REFERENCES item (id),
    name            TEXT    NOT NULL,
    picture         TEXT    NOT NULL,
    price           INTEGER NOT NULL CHECK (price >= 0),
    status          TEXT    NOT NULL CHECK (status IN ('active', 'handed_over', 'given_up'))
) STRICT;

-- At most one active goal per family member.
CREATE UNIQUE INDEX goal_one_active ON goal (member_id) WHERE status = 'active';
