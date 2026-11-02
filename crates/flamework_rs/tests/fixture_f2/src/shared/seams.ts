import { Modding } from "@flamework-experimental/core";
import { constant, emit, fieldInfo, textOf, where } from "./markers";

// Strings the TS transformer makes with `f.string` print as TypeScript's printer writes a synthesized literal:
// non-ASCII as `\u{…}`, quotes and control characters escaped by name.
export const typeText = textOf<'say "hi" – ünï'>();
export const callerText = where(/* "é" */);
export const emitted = emit<{ naïve: "ünï"; plain: 'a"b'; controls: "\t\v" }>();

// Properties in TypeScript 7.1's order: a derived interface's own members first, then the inherited ones; a mapped
// type over it follows the declarations.
interface Base {
	base: 1;
	shared: 2;
}
interface Child extends Base {
	own: 3;
	shared: 2;
}
export const ordered = emit<Child>();
export const orderedFields = fieldInfo<Child>();
export const orderedMapped = emit<Readonly<Child>>();
export const orderedMappedFields = fieldInfo<Readonly<Child>>();

// The "hoisted" comment goes ahead of the TS statement around the call (the TS's `state.prereq`).
export function conditional(flag: boolean) {
	const value = flag ? constant<{ deep: 1 }>() : undefined;
	return value;
}

export function shortCircuit(flag: boolean) {
	return flag && [constant<{ deeper: 2 }>()];
}

// `{@link}` keys are the link's text as TypeScript 5 parses it.
/** @metadata macro {@link text  intrinsic-inline} */
export function spaced<T>(text?: Modding.Target.Text<T>): string {
	return text!;
}

/** @metadata macro {@link text | intrinsic-inline} */
export function piped<T>(text?: Modding.Target.Text<T>): string {
	return text!;
}

/** @metadata macro {@link text intrinsic-inline } */
export function trailing<T>(text?: Modding.Target.Text<T>): string {
	return text!;
}

export const spacedText = spaced<"s">();
export const pipedText = piped<"p">();
export const trailingText = trailing<"t">();
