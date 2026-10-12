// Island open/close state machine (src/island/fsm.ts).

import { afterEach, beforeEach, mock, test } from "node:test";
import assert from "node:assert/strict";
import { IslandStateMachine } from "../src/island/fsm.ts";

let fsm;
let transitions;

beforeEach(() => {
  mock.timers.enable({ apis: ["setTimeout"] });
  fsm = new IslandStateMachine();
  transitions = [];
  fsm.onTransition = (from, to) => transitions.push(`${from}>${to}`);
});

afterEach(() => mock.timers.reset());

const seconds = (n) => mock.timers.tick(n * 1000);

test("a completion waits for dismissal despite normal auto-close and hover timers", () => {
  fsm.openOnHover = true; fsm.mouseEntered(); fsm.mouseLeft();
  fsm.setCompletion(0);
  seconds(86400);
  assert.equal(fsm.state, "home");
  fsm.applyVisibility(false, false, 60);
  seconds(600);
  assert.equal(fsm.state, "home");
  fsm.forcePetit();
  assert.equal(fsm.completionDelay, null);
  assert.equal(fsm.state, "petit");
});

test("completion uses its own selected timer and leaves ordinary visibility unchanged", () => {
  fsm.keepExpanded = true;
  fsm.forceHome(); fsm.setCompletion(30); fsm.mouseLeft();
  seconds(29); assert.equal(fsm.state, "home");
  seconds(1); assert.equal(fsm.state, "petit");
  assert.equal(fsm.completionDelay, null);
  fsm.forceHome(); fsm.mouseLeft(); seconds(600);
  assert.equal(fsm.state, "home", "the normal keep-open preference is retained");
});

test("changing a completion duration applies at once and pending approvals remain pinned", () => {
  fsm.forceHome(); fsm.setCompletion(0); fsm.mouseLeft();
  fsm.setCompletion(5); fsm.pinned = true;
  seconds(5); assert.equal(fsm.state, "home");
  fsm.pinned = false; fsm.mouseLeft();
  seconds(5); assert.equal(fsm.state, "petit");
});

test("starts hidden and opens on the greeting at launch", () => {
  assert.equal(fsm.state, "hidden");
  fsm.launch();
  assert.equal(fsm.state, "coucou");
  assert.deepEqual(transitions, ["hidden>coucou"]);
});

test("the greeting collapses to the compact island 0.6 s after it ends", () => {
  fsm.launch();
  fsm.greetComplete();
  seconds(0.5);
  assert.equal(fsm.state, "coucou");
  seconds(0.1);
  assert.equal(fsm.state, "petit");
});

test("a hovered greeting stays for 10 s, whatever the animation does", () => {
  fsm.launch();
  fsm.mouseEntered();
  fsm.greetComplete();
  seconds(9.9);
  assert.equal(fsm.state, "coucou");
  seconds(0.1);
  assert.equal(fsm.state, "petit");
});

test("leaving the greeting collapses it at once", () => {
  fsm.launch();
  fsm.mouseEntered();
  fsm.mouseLeft();
  assert.equal(fsm.state, "petit");
});

test("the mouse wakes a hidden island, which stays while it is hovered", () => {
  fsm.mouseEntered();
  assert.equal(fsm.state, "petit");
  seconds(600);
  assert.equal(fsm.state, "petit");
});

test("the compact island hides 60 s after the mouse leaves", () => {
  fsm.mouseEntered();
  fsm.mouseLeft();
  seconds(59);
  assert.equal(fsm.state, "petit");
  seconds(1);
  assert.equal(fsm.state, "hidden");
});

test("coming back before the 60 s are up cancels the hide", () => {
  fsm.mouseEntered();
  fsm.mouseLeft();
  seconds(59);
  fsm.mouseEntered();
  seconds(600);
  assert.equal(fsm.state, "petit");
});

test("a click opens the compact island, and only the compact island", () => {
  fsm.click();
  assert.equal(fsm.state, "hidden");
  fsm.mouseEntered();
  fsm.click();
  assert.equal(fsm.state, "home");
  fsm.click();
  assert.equal(fsm.state, "home");
  assert.deepEqual(transitions, ["hidden>petit", "petit>home"]);
});

test("the open island collapses 15 s after the mouse leaves", () => {
  fsm.forceHome();
  fsm.mouseLeft();
  seconds(14);
  assert.equal(fsm.state, "home");
  seconds(1);
  assert.equal(fsm.state, "petit");
});

test("coming back to the open island cancels the collapse", () => {
  fsm.forceHome();
  fsm.mouseLeft();
  seconds(14);
  fsm.mouseEntered();
  seconds(600);
  assert.equal(fsm.state, "home");
});

test("a pinned island stays open when the mouse leaves", () => {
  fsm.forceHome();
  fsm.pinned = true;
  fsm.mouseLeft();
  seconds(600);
  assert.equal(fsm.state, "home");
});

test("the collapse delay is the configured one", () => {
  fsm.homeToPetitDelay = 5;
  fsm.forceHome();
  fsm.mouseLeft();
  seconds(5);
  assert.equal(fsm.state, "petit");
});

test("reveal shows the compact island from hidden and hides it again after 60 s", () => {
  fsm.reveal();
  assert.equal(fsm.state, "petit");
  seconds(60);
  assert.equal(fsm.state, "hidden");
});

test("reveal leaves an island that is already showing alone", () => {
  fsm.forceHome();
  fsm.reveal();
  assert.equal(fsm.state, "home");
  assert.deepEqual(transitions, ["hidden>home"]);
});

test("forcing the island open cancels a pending hide", () => {
  fsm.reveal();
  fsm.forceHome();
  seconds(600);
  assert.equal(fsm.state, "home");
});

test("an explicit close goes to the compact island and cancels the collapse", () => {
  fsm.forceHome();
  fsm.mouseLeft();
  fsm.forcePetit();
  assert.equal(fsm.state, "petit");
  seconds(600);
  assert.equal(fsm.state, "petit");
});

test("forceHidden hides from any state", () => {
  fsm.forceHome();
  fsm.forceHidden();
  assert.equal(fsm.state, "hidden");
});

test("a transition to the current state is not reported", () => {
  fsm.forceHome();
  fsm.forceHome();
  assert.deepEqual(transitions, ["hidden>home"]);
});

// ── Auto-close delay from Settings (IslandAutoCloseTests.swift) ──────────────

/** Opened by a click, the way the Mac tests open theirs. */
function opened(delay) {
  fsm.homeToPetitDelay = delay;
  fsm.mouseEntered();
  fsm.click();
  assert.equal(fsm.state, "home");
}

test("a configured 2-second delay replaces the default 15 seconds", () => {
  opened(2);
  fsm.mouseLeft();
  seconds(1.9);
  assert.equal(fsm.state, "home");
  seconds(0.1);
  assert.equal(fsm.state, "petit");
});

test("editing the delay during a countdown replaces its timer", () => {
  opened(15);
  fsm.mouseLeft();
  fsm.homeToPetitDelay = 0.05;
  seconds(0.05);
  assert.equal(fsm.state, "petit");
});

test("a longer delay also cancels the shorter timer that was running", () => {
  opened(0.05);
  fsm.mouseLeft();
  fsm.homeToPetitDelay = 0.25;
  seconds(0.1);
  assert.equal(fsm.state, "home");
  seconds(0.15);
  assert.equal(fsm.state, "petit");
});

test("coming back cancels the countdown, and leaving again starts it with the delay", () => {
  opened(0.05);
  fsm.mouseLeft();
  fsm.mouseEntered();
  seconds(0.1);
  assert.equal(fsm.state, "home");
  fsm.mouseLeft();
  seconds(0.05);
  assert.equal(fsm.state, "petit");
});

test("a delay edit while the island is hovered starts no countdown", () => {
  opened(15);
  fsm.homeToPetitDelay = 0.05;
  seconds(600);
  assert.equal(fsm.state, "home");
  fsm.mouseLeft();
  seconds(0.05);
  assert.equal(fsm.state, "petit");
});

test("the greeting keeps its own timing whatever the auto-close delay", () => {
  fsm.greetAutoCollapseDelay = 0.15;
  fsm.launch();
  fsm.greetComplete();
  fsm.homeToPetitDelay = 0.01;
  seconds(0.05);
  assert.equal(fsm.state, "coucou");
  seconds(0.1);
  assert.equal(fsm.state, "petit");
});

test("an alert waiting for an answer stays open through a delay edit", () => {
  opened(0.05);
  fsm.pinned = true;
  fsm.mouseLeft();
  fsm.homeToPetitDelay = 0.01;
  seconds(600);
  assert.equal(fsm.state, "home");
  fsm.pinned = false;
  fsm.mouseLeft();
  seconds(0.01);
  assert.equal(fsm.state, "petit");
});

test("an alert pinned during a countdown blocks the old timer and its replacement", () => {
  opened(0.2);
  fsm.mouseLeft();
  fsm.pinned = true;
  fsm.homeToPetitDelay = 0.01;
  seconds(600);
  assert.equal(fsm.state, "home");
  fsm.pinned = false;
  fsm.mouseLeft();
  seconds(0.01);
  assert.equal(fsm.state, "petit");
});

test("a pin that arrives after the timer was armed still holds the island", () => {
  opened(1);
  fsm.mouseLeft();
  fsm.pinned = true;
  seconds(600);
  assert.equal(fsm.state, "home");
});

test("the deadline the countdown bar reads follows the delay and clears with the timer", () => {
  opened(15);
  assert.equal(fsm.homeCollapseDueAt, null);
  fsm.mouseLeft();
  const first = fsm.homeCollapseDueAt;
  assert.ok(first != null);
  fsm.homeToPetitDelay = 5;
  assert.ok(fsm.homeCollapseDueAt < first);
  fsm.mouseEntered();
  assert.equal(fsm.homeCollapseDueAt, null);
});

// ── A card folded away while it waits (Mac #290) ─────────────────────────────

test("a folded card keeps the compact island on screen until it is answered", () => {
  fsm.forceHome();
  fsm.pinned = true;
  fsm.forcePetit();
  fsm.mouseLeft();
  seconds(600);
  assert.equal(fsm.state, "petit");
  // Reopening brings it back open, and the mouse leaving does not fold it.
  fsm.mouseEntered();
  fsm.click();
  fsm.mouseLeft();
  seconds(600);
  assert.equal(fsm.state, "home");
  // Answered: the usual timers again.
  fsm.pinned = false;
  fsm.mouseLeft();
  seconds(15);
  assert.equal(fsm.state, "petit");
  fsm.mouseLeft();
  seconds(60);
  assert.equal(fsm.state, "hidden");
});

test("an unusable delay is ignored", () => {
  for (const bad of [NaN, -1, Infinity]) fsm.homeToPetitDelay = bad;
  assert.equal(fsm.homeToPetitDelay, 15);
});

// ── Open on hover (IslandHoverTests.swift) ────────────────────────────────────

test("open on hover off: hovering only peeks", () => {
  fsm.mouseEntered();
  assert.equal(fsm.state, "petit");
  assert.equal(fsm.openedByHover, false);
});

test("open on hover: hovering opens, leaving folds after the short grace period", () => {
  fsm.openOnHover = true;
  fsm.mouseEntered();
  assert.equal(fsm.state, "home");
  assert.equal(fsm.openedByHover, true);
  fsm.mouseLeft();
  seconds(0.5);
  assert.equal(fsm.state, "home");
  seconds(0.1);
  assert.equal(fsm.state, "petit");
  assert.equal(fsm.openedByHover, false);
  // From the compact island it opens again.
  fsm.mouseEntered();
  assert.equal(fsm.state, "home");
});

test("open on hover: coming back before the grace period keeps it open", () => {
  fsm.openOnHover = true;
  fsm.mouseEntered();
  fsm.mouseLeft();
  seconds(0.3);
  fsm.mouseEntered();
  seconds(5);
  assert.equal(fsm.state, "home");
});

test("open on hover: a click inside makes it an ordinary open island", () => {
  fsm.openOnHover = true;
  fsm.homeToPetitDelay = 3;
  fsm.mouseEntered();
  fsm.mouseLeft();
  fsm.userInteracted();          // the short countdown becomes the normal one
  seconds(1);
  assert.equal(fsm.state, "home");
  seconds(2);
  assert.equal(fsm.state, "petit");
});

test("open on hover: a waiting card holds the island, hover neither opens nor folds it", () => {
  fsm.openOnHover = true;
  fsm.pinned = true;
  fsm.mouseEntered();
  assert.equal(fsm.state, "petit");
  assert.equal(fsm.openedByHover, false);
  fsm.forceHome();
  fsm.mouseLeft();
  seconds(5);
  assert.equal(fsm.state, "home");
});

test("open on hover: an island opened by an alert keeps the normal delay", () => {
  fsm.openOnHover = true;
  fsm.homeToPetitDelay = 3;
  fsm.forceHome();
  fsm.mouseEntered();
  fsm.mouseLeft();
  seconds(1);
  assert.equal(fsm.state, "home");
  seconds(2);
  assert.equal(fsm.state, "petit");
});

test("hover opening respects the fork's keep-open and keep-minimized preferences", () => {
  fsm.openOnHover = true;
  fsm.applyVisibility(true, true, 120);
  fsm.mouseEntered();
  fsm.mouseLeft();
  seconds(600);
  assert.equal(fsm.state, "home", "keep-open overrides the hover grace period");
  fsm.forcePetit();
  fsm.mouseLeft();
  seconds(600);
  assert.equal(fsm.state, "petit", "explicit minimize remains visible");
  fsm.applyVisibility(false, false, 120);
  seconds(119);
  assert.equal(fsm.state, "petit");
  seconds(1);
  assert.equal(fsm.state, "hidden", "custom hide timer still applies");
});

test("unpinning a hover-opened view uses the hover delay unless keep-open is enabled", () => {
  fsm.openOnHover = true;
  fsm.mouseEntered();
  fsm.pinned = true;
  fsm.mouseLeft();
  seconds(60);
  assert.equal(fsm.state, "home");
  fsm.pinned = false;
  fsm.mouseLeft();
  seconds(0.6);
  assert.equal(fsm.state, "petit");
});
