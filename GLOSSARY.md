# Pebblepot

A single household's system for rewarding kids for chores and good behaviour with a currency they save up and spend.

## People

**Household**:
The one family the app serves. There is exactly one.
_Avoid_: Family account, tenant, organization

**Family member**:
A person in the household, either a Parent or a Kid. Every family member has a pot and can earn and redeem.
_Avoid_: User, account, profile

**Parent**:
A family member who looks after the chore, good deed and reward lists. Parents earn pebbles too, so the kids see the grown-ups pitching in.
_Avoid_: Admin, guardian

**Kid**:
A family member who isn't a parent: one of the children the app is mainly for.
_Avoid_: Child, user, player

## Currency

**Pebble**:
The unit of reward currency. Pebbles are only ever gained or spent, never taken away as punishment.
_Avoid_: Point, star, token, coin

**Pot**:
A family member's running balance of pebbles. Only a goal can take it below zero.
_Avoid_: Wallet, account, balance, bank

**Earn**:
To have pebbles added to a pot, for a completed chore or a good deed.
_Avoid_: Award, credit

**Redeem**:
To spend pebbles from a pot on a reward, taking effect immediately.
_Avoid_: Buy, cash in, withdraw

**Log**:
The record of every pebble earned and redeemed: which family member, what for, how many, and when. Each entry keeps its own copy of the name, picture and amount at the time. Today's entries can still be undone; once the day is over the log is permanent. A pot's balance is what its log adds up to.
_Avoid_: History, ledger, transactions, activity

**Undo**:
Removing an entry from the log on the same day it was made. Earlier days can't be undone.
_Avoid_: Strike, reverse, refund, delete

## Earning

**Chore**:
A predefined task with a fixed pebble value that any family member can do, at most once per day each; doing it earns the pebbles straight away.
_Avoid_: Task, job

**Daily chore**:
A chore on the short everyday list, picked with a single tap.
_Avoid_: Recurring chore, scheduled chore

**Occasional chore**:
A chore on the longer, less frequent list, below the daily chores and good deeds and filterable by search.
_Avoid_: One-off chore, extra chore

**Good deed**:
An act of good behaviour, picked from a list or written in free form with a note, that earns pebbles on the spot.
_Avoid_: Good behaviour, bonus, kudos

**Retire**:
To take a chore, good deed, reward or family member off the board for good, without touching any log entries that mention it.
_Avoid_: Delete, archive, remove

## Spending

**Reward**:
An item on the household's list of things a family member can redeem pebbles for, each with a pebble price.
_Avoid_: Prize, treat, item

**Goal**:
A reward redeemed before its owner can afford it, so their pot goes negative by the shortfall. It is handed over once the pot is back to zero or more. A family member has at most one goal at a time and can give it up for a full refund.
_Avoid_: Wishlist, target, savings goal, layaway
