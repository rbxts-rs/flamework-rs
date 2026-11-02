import { Flamework } from "@flamework-experimental/core";

// Strings the TS builder makes with `f.string` (patterns, literals, keys) print as TypeScript's printer writes a
// synthesized literal: non-ASCII as `\u{…}`, quotes and control characters escaped by name.
export const template = Flamework.createGuard<`é-${string}-✓`>();
export const astral = Flamework.createGuard<`😀${number}`>();
export const literal = Flamework.createGuard<"naïve">();
export const controls = Flamework.createGuard<"\v\f\b\0">();
export const keys = Flamework.createGuard<{ naïve: number; 日本: string; 'a"b': boolean }>();
export const childNames = Flamework.createGuard<Model & { Ünï: Part }>();

// Properties in TypeScript 7.1's order: a derived interface's own members first, then the inherited ones; a mapped
// type over it and a spread follow the declarations.
interface Base {
	base: string;
	shared: number;
}
interface Child extends Base {
	own: boolean;
	shared: number;
}
interface Holder extends Model {
	Zeta: Folder;
}
interface DerivedHolder extends Holder {
	Alpha: Part;
}
export const inherited = Flamework.createGuard<Child>();
export const inheritedChildren = Flamework.createGuard<DerivedHolder>();
export const mapped = Flamework.createGuard<Partial<Child>>();
const spreadSource = { ...{ q: 1, p: "x" }, a: true, q: 2 };
export const spread = Flamework.createGuard<typeof spreadSource>();
export const dedupKeys = Flamework.createGuard<{ z: Readonly<Child>; y: Readonly<Child>; x: Readonly<Child> }>();
