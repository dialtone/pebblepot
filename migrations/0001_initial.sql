CREATE TABLE family_member (
    id       INTEGER PRIMARY KEY,
    name     TEXT    NOT NULL,
    face     TEXT    NOT NULL,
    colour   TEXT    NOT NULL,
    role     TEXT    NOT NULL CHECK (role IN ('kid', 'parent')),
    position INTEGER NOT NULL,
    retired  INTEGER NOT NULL DEFAULT 0 CHECK (retired IN (0, 1))
) STRICT;

-- Chores, good deeds and rewards. `value` is the pebbles earned, or the price for a reward.
CREATE TABLE item (
    id       INTEGER PRIMARY KEY,
    kind     TEXT    NOT NULL CHECK (kind IN ('daily_chore', 'occasional_chore', 'good_deed', 'reward')),
    name     TEXT    NOT NULL,
    picture  TEXT    NOT NULL,
    value    INTEGER NOT NULL CHECK (value >= 0),
    position INTEGER NOT NULL,
    retired  INTEGER NOT NULL DEFAULT 0 CHECK (retired IN (0, 1))
) STRICT;

-- The log. Each entry keeps its own copy of the name, picture and amount, so editing
-- or retiring an item never rewrites history. A pot is the sum of its member's entries.
CREATE TABLE log_entry (
    id         INTEGER PRIMARY KEY,
    member_id  INTEGER NOT NULL REFERENCES family_member (id),
    entry_type TEXT    NOT NULL CHECK (entry_type IN ('earn', 'redeem', 'goal_set', 'goal_refund')),
    item_id    INTEGER REFERENCES item (id),
    name       TEXT    NOT NULL,
    picture    TEXT    NOT NULL,
    amount     INTEGER NOT NULL,
    note       TEXT,
    local_day  TEXT    NOT NULL,
    created_at TEXT    NOT NULL
) STRICT;

CREATE INDEX log_entry_member_day ON log_entry (member_id, local_day);
