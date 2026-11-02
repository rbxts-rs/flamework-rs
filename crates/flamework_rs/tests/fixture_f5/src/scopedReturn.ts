import { Events } from "./client";

declare function compute(): number;

// A returned send past the function's budget of locals: TS5 puts the return statement in a do block with its packing,
// flamework-rs packs it in a function (README, Differences from the TS transformer).
export function scopedReturn(value: number) {
	Events.bravo.fire(`n${value + 0}`, compute());
	Events.bravo.fire(`n${value + 1}`, compute());
	Events.bravo.fire(`n${value + 2}`, compute());
	Events.bravo.fire(`n${value + 3}`, compute());
	Events.bravo.fire(`n${value + 4}`, compute());
	Events.bravo.fire(`n${value + 5}`, compute());
	Events.bravo.fire(`n${value + 6}`, compute());
	Events.bravo.fire(`n${value + 7}`, compute());
	Events.bravo.fire(`n${value + 8}`, compute());
	Events.bravo.fire(`n${value + 9}`, compute());
	Events.bravo.fire(`n${value + 10}`, compute());
	Events.bravo.fire(`n${value + 11}`, compute());
	Events.bravo.fire(`n${value + 12}`, compute());
	Events.bravo.fire(`n${value + 13}`, compute());
	Events.bravo.fire(`n${value + 14}`, compute());
	Events.bravo.fire(`n${value + 15}`, compute());
	Events.bravo.fire(`n${value + 16}`, compute());
	Events.bravo.fire(`n${value + 17}`, compute());
	Events.bravo.fire(`n${value + 18}`, compute());
	Events.bravo.fire(`n${value + 19}`, compute());
	return Events.echo.fire(compute(), compute(), compute(), compute(), compute(), compute(), compute(), compute(), compute(), compute());
}
