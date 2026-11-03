import { Flamework } from "@flamework-experimental/core";

/** More literal groups and enums than the TS transformer's 16 file locals: all of them locals here. */
export const l1 = Flamework.createSerializer<"a1" | "b1">();
export const l2 = Flamework.createSerializer<"a2" | "b2">();
export const l3 = Flamework.createSerializer<"a3" | "b3">();
export const l4 = Flamework.createSerializer<"a4" | "b4">();
export const l5 = Flamework.createSerializer<"a5" | "b5">();
export const l6 = Flamework.createSerializer<"a6" | "b6">();
export const l7 = Flamework.createSerializer<"a7" | "b7">();
export const l8 = Flamework.createSerializer<"a8" | "b8">();
export const l9 = Flamework.createSerializer<"a9" | "b9">();
export const e1 = Flamework.createSerializer<Enum.Material>();
export const e2 = Flamework.createSerializer<Enum.KeyCode>();
export const e3 = Flamework.createSerializer<Enum.UserInputType>();
export const l10 = Flamework.createSerializer<"a10" | "b10">();
