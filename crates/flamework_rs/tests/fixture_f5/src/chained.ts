import { Events, Functions } from "./client";

declare function compute(): number;
declare function pick(): typeof Events | undefined;
declare function key(): string;

interface Holder {
	ev?: typeof Events;
	fns?: typeof Functions;
	alpha?: (typeof Events)["alpha"];
	inner?: { ev?: typeof Events };
}

// Sends through `?.`, packed inside the compiler's nil check: each operand ahead of a `?.` is read once, into a
// local, before the arguments, and the send goes through it (JavaScript's order). The TS transformer checks the
// operands, packs, and then reads the chain again for the send.
export function chained(maybe: typeof Events | undefined, value: number) {
	maybe?.alpha.fire(value * 3);
	maybe?.charlie.fire();
}

// An argument that clears the operand ahead of the `?.` does not change the send (it was read before it).
export function cleared(holder: Holder) {
	holder.ev?.alpha.fire(
		(() => {
			holder.ev = undefined;
			return 1;
		})(),
	);
}

// Other shapes: deeper, two `?.`, `?.` before the member called, the member called itself, a key with effects, a
// call operand, setCallback, a value used, invoke.
export function shapes(holder: Holder, store: Record<string, typeof Events | undefined>) {
	holder.ev?.juliet.kilo.fire(compute() > 0);
	holder.inner?.ev?.bravo.fire("both", compute());
	holder.alpha?.fire(compute());
	holder.ev?.alpha(compute());
	store[key()]?.alpha.fire(compute());
	pick()?.alpha.fire(compute());
	holder.fns?.romeo.setCallback((value) => `${value}`);
	const sent = holder.ev?.alpha.fire(compute());
	return [sent, holder.fns?.november.invoke(compute())];
}

// At the top of the file.
declare const top: Holder;
top.ev?.alpha.fire(compute());
