import { Flamework } from "@flamework-experimental/core";
import { Events } from "./network";

declare const text: string;

/** Literal groups: a list and an index each. */
export const literals = [
	Flamework.createSerializer<"a0" | "b0" | "c0">(),
	Flamework.createSerializer<"a1" | "b1" | "c1">(),
	Flamework.createSerializer<"a2" | "b2" | "c2">(),
	Flamework.createSerializer<"a3" | "b3" | "c3">(),
	Flamework.createSerializer<"a4" | "b4" | "c4">(),
	Flamework.createSerializer<"a5" | "b5" | "c5">(),
	Flamework.createSerializer<"a6" | "b6" | "c6">(),
	Flamework.createSerializer<"a7" | "b7" | "c7">(),
	Flamework.createSerializer<"a8" | "b8" | "c8">(),
	Flamework.createSerializer<"a9" | "b9" | "c9">(),
	Flamework.createSerializer<"a10" | "b10" | "c10">(),
	Flamework.createSerializer<"a11" | "b11" | "c11">(),
	Flamework.createSerializer<"a12" | "b12" | "c12">(),
	Flamework.createSerializer<"a13" | "b13" | "c13">(),
	Flamework.createSerializer<"a14" | "b14" | "c14">(),
	Flamework.createSerializer<"a15" | "b15" | "c15">(),
	Flamework.createSerializer<"a16" | "b16" | "c16">(),
	Flamework.createSerializer<"a17" | "b17" | "c17">(),
	Flamework.createSerializer<"a18" | "b18" | "c18">(),
	Flamework.createSerializer<"a19" | "b19" | "c19">(),
	Flamework.createSerializer<"a20" | "b20" | "c20">(),
	Flamework.createSerializer<"a21" | "b21" | "c21">(),
	Flamework.createSerializer<"a22" | "b22" | "c22">(),
	Flamework.createSerializer<"a23" | "b23" | "c23">(),
	Flamework.createSerializer<"a24" | "b24" | "c24">(),
	Flamework.createSerializer<"a25" | "b25" | "c25">(),
	Flamework.createSerializer<"a26" | "b26" | "c26">(),
	Flamework.createSerializer<"a27" | "b27" | "c27">(),
	Flamework.createSerializer<"a28" | "b28" | "c28">(),
	Flamework.createSerializer<"a29" | "b29" | "c29">(),
	Flamework.createSerializer<"a30" | "b30" | "c30">(),
	Flamework.createSerializer<"a31" | "b31" | "c31">(),
	Flamework.createSerializer<"a32" | "b32" | "c32">(),
	Flamework.createSerializer<"a33" | "b33" | "c33">(),
	Flamework.createSerializer<"a34" | "b34" | "c34">(),
	Flamework.createSerializer<"a35" | "b35" | "c35">(),
	Flamework.createSerializer<"a36" | "b36" | "c36">(),
	Flamework.createSerializer<"a37" | "b37" | "c37">(),
	Flamework.createSerializer<"a38" | "b38" | "c38">(),
	Flamework.createSerializer<"a39" | "b39" | "c39">(),
	Flamework.createSerializer<"a40" | "b40" | "c40">(),
	Flamework.createSerializer<"a41" | "b41" | "c41">(),
	Flamework.createSerializer<"a42" | "b42" | "c42">(),
	Flamework.createSerializer<"a43" | "b43" | "c43">(),
	Flamework.createSerializer<"a44" | "b44" | "c44">(),
	Flamework.createSerializer<"a45" | "b45" | "c45">(),
	Flamework.createSerializer<"a46" | "b46" | "c46">(),
	Flamework.createSerializer<"a47" | "b47" | "c47">(),
	Flamework.createSerializer<"a48" | "b48" | "c48">(),
	Flamework.createSerializer<"a49" | "b49" | "c49">(),
	Flamework.createSerializer<"a50" | "b50" | "c50">(),
	Flamework.createSerializer<"a51" | "b51" | "c51">(),
	Flamework.createSerializer<"a52" | "b52" | "c52">(),
	Flamework.createSerializer<"a53" | "b53" | "c53">(),
	Flamework.createSerializer<"a54" | "b54" | "c54">(),
	Flamework.createSerializer<"a55" | "b55" | "c55">(),
	Flamework.createSerializer<"a56" | "b56" | "c56">(),
	Flamework.createSerializer<"a57" | "b57" | "c57">(),
	Flamework.createSerializer<"a58" | "b58" | "c58">(),
	Flamework.createSerializer<"a59" | "b59" | "c59">(),
	Flamework.createSerializer<"a60" | "b60" | "c60">(),
	Flamework.createSerializer<"a61" | "b61" | "c61">(),
	Flamework.createSerializer<"a62" | "b62" | "c62">(),
	Flamework.createSerializer<"a63" | "b63" | "c63">(),
	Flamework.createSerializer<"a64" | "b64" | "c64">(),
	Flamework.createSerializer<"a65" | "b65" | "c65">(),
	Flamework.createSerializer<"a66" | "b66" | "c66">(),
	Flamework.createSerializer<"a67" | "b67" | "c67">(),
	Flamework.createSerializer<"a68" | "b68" | "c68">(),
	Flamework.createSerializer<"a69" | "b69" | "c69">(),
	Flamework.createSerializer<"a70" | "b70" | "c70">(),
	Flamework.createSerializer<"a71" | "b71" | "c71">(),
	Flamework.createSerializer<"a72" | "b72" | "c72">(),
	Flamework.createSerializer<"a73" | "b73" | "c73">(),
	Flamework.createSerializer<"a74" | "b74" | "c74">(),
	Flamework.createSerializer<"a75" | "b75" | "c75">(),
	Flamework.createSerializer<"a76" | "b76" | "c76">(),
	Flamework.createSerializer<"a77" | "b77" | "c77">(),
	Flamework.createSerializer<"a78" | "b78" | "c78">(),
	Flamework.createSerializer<"a79" | "b79" | "c79">(),
];

/** Unions told apart by a guard. */
export const unions = [
	Flamework.createSerializer<Partial<{ p0: number }> | Partial<{ q0: string }>>(),
	Flamework.createSerializer<Partial<{ p1: number }> | Partial<{ q1: string }>>(),
	Flamework.createSerializer<Partial<{ p2: number }> | Partial<{ q2: string }>>(),
	Flamework.createSerializer<Partial<{ p3: number }> | Partial<{ q3: string }>>(),
	Flamework.createSerializer<Partial<{ p4: number }> | Partial<{ q4: string }>>(),
	Flamework.createSerializer<Partial<{ p5: number }> | Partial<{ q5: string }>>(),
	Flamework.createSerializer<Partial<{ p6: number }> | Partial<{ q6: string }>>(),
	Flamework.createSerializer<Partial<{ p7: number }> | Partial<{ q7: string }>>(),
	Flamework.createSerializer<Partial<{ p8: number }> | Partial<{ q8: string }>>(),
	Flamework.createSerializer<Partial<{ p9: number }> | Partial<{ q9: string }>>(),
	Flamework.createSerializer<Partial<{ p10: number }> | Partial<{ q10: string }>>(),
	Flamework.createSerializer<Partial<{ p11: number }> | Partial<{ q11: string }>>(),
	Flamework.createSerializer<Partial<{ p12: number }> | Partial<{ q12: string }>>(),
	Flamework.createSerializer<Partial<{ p13: number }> | Partial<{ q13: string }>>(),
	Flamework.createSerializer<Partial<{ p14: number }> | Partial<{ q14: string }>>(),
	Flamework.createSerializer<Partial<{ p15: number }> | Partial<{ q15: string }>>(),
	Flamework.createSerializer<Partial<{ p16: number }> | Partial<{ q16: string }>>(),
	Flamework.createSerializer<Partial<{ p17: number }> | Partial<{ q17: string }>>(),
	Flamework.createSerializer<Partial<{ p18: number }> | Partial<{ q18: string }>>(),
	Flamework.createSerializer<Partial<{ p19: number }> | Partial<{ q19: string }>>(),
	Flamework.createSerializer<Partial<{ p20: number }> | Partial<{ q20: string }>>(),
	Flamework.createSerializer<Partial<{ p21: number }> | Partial<{ q21: string }>>(),
	Flamework.createSerializer<Partial<{ p22: number }> | Partial<{ q22: string }>>(),
	Flamework.createSerializer<Partial<{ p23: number }> | Partial<{ q23: string }>>(),
	Flamework.createSerializer<Partial<{ p24: number }> | Partial<{ q24: string }>>(),
	Flamework.createSerializer<Partial<{ p25: number }> | Partial<{ q25: string }>>(),
	Flamework.createSerializer<Partial<{ p26: number }> | Partial<{ q26: string }>>(),
	Flamework.createSerializer<Partial<{ p27: number }> | Partial<{ q27: string }>>(),
	Flamework.createSerializer<Partial<{ p28: number }> | Partial<{ q28: string }>>(),
	Flamework.createSerializer<Partial<{ p29: number }> | Partial<{ q29: string }>>(),
	Flamework.createSerializer<Partial<{ p30: number }> | Partial<{ q30: string }>>(),
	Flamework.createSerializer<Partial<{ p31: number }> | Partial<{ q31: string }>>(),
	Flamework.createSerializer<Partial<{ p32: number }> | Partial<{ q32: string }>>(),
	Flamework.createSerializer<Partial<{ p33: number }> | Partial<{ q33: string }>>(),
	Flamework.createSerializer<Partial<{ p34: number }> | Partial<{ q34: string }>>(),
	Flamework.createSerializer<Partial<{ p35: number }> | Partial<{ q35: string }>>(),
	Flamework.createSerializer<Partial<{ p36: number }> | Partial<{ q36: string }>>(),
	Flamework.createSerializer<Partial<{ p37: number }> | Partial<{ q37: string }>>(),
	Flamework.createSerializer<Partial<{ p38: number }> | Partial<{ q38: string }>>(),
	Flamework.createSerializer<Partial<{ p39: number }> | Partial<{ q39: string }>>(),
];

/** Enum tables. */
export const enums = [
	Flamework.createSerializer<Enum.Material>(),
	Flamework.createSerializer<Enum.KeyCode>(),
	Flamework.createSerializer<Enum.UserInputType>(),
	Flamework.createSerializer<Enum.Font>(),
	Flamework.createSerializer<Enum.PartType>(),
];

/** Dedup guards in the main chunk: each interface met twice. */
interface D0 {
	x0: number;
}
interface D1 {
	x1: number;
}
interface D2 {
	x2: number;
}
interface D3 {
	x3: number;
}
interface D4 {
	x4: number;
}
interface D5 {
	x5: number;
}
interface D6 {
	x6: number;
}
interface D7 {
	x7: number;
}
interface D8 {
	x8: number;
}
interface D9 {
	x9: number;
}
interface D10 {
	x10: number;
}
interface D11 {
	x11: number;
}
interface D12 {
	x12: number;
}
interface D13 {
	x13: number;
}
interface D14 {
	x14: number;
}
interface D15 {
	x15: number;
}
interface D16 {
	x16: number;
}
interface D17 {
	x17: number;
}
interface D18 {
	x18: number;
}
interface D19 {
	x19: number;
}
interface D20 {
	x20: number;
}
interface D21 {
	x21: number;
}
interface D22 {
	x22: number;
}
interface D23 {
	x23: number;
}
interface D24 {
	x24: number;
}
interface D25 {
	x25: number;
}
interface D26 {
	x26: number;
}
interface D27 {
	x27: number;
}
interface D28 {
	x28: number;
}
interface D29 {
	x29: number;
}
interface D30 {
	x30: number;
}
interface D31 {
	x31: number;
}
interface D32 {
	x32: number;
}
interface D33 {
	x33: number;
}
interface D34 {
	x34: number;
}
interface D35 {
	x35: number;
}
interface D36 {
	x36: number;
}
interface D37 {
	x37: number;
}
interface D38 {
	x38: number;
}
interface D39 {
	x39: number;
}
export const guards = [
	Flamework.createGuard<{ a: D0; b: D0 }>(),
	Flamework.createGuard<{ a: D1; b: D1 }>(),
	Flamework.createGuard<{ a: D2; b: D2 }>(),
	Flamework.createGuard<{ a: D3; b: D3 }>(),
	Flamework.createGuard<{ a: D4; b: D4 }>(),
	Flamework.createGuard<{ a: D5; b: D5 }>(),
	Flamework.createGuard<{ a: D6; b: D6 }>(),
	Flamework.createGuard<{ a: D7; b: D7 }>(),
	Flamework.createGuard<{ a: D8; b: D8 }>(),
	Flamework.createGuard<{ a: D9; b: D9 }>(),
	Flamework.createGuard<{ a: D10; b: D10 }>(),
	Flamework.createGuard<{ a: D11; b: D11 }>(),
	Flamework.createGuard<{ a: D12; b: D12 }>(),
	Flamework.createGuard<{ a: D13; b: D13 }>(),
	Flamework.createGuard<{ a: D14; b: D14 }>(),
	Flamework.createGuard<{ a: D15; b: D15 }>(),
	Flamework.createGuard<{ a: D16; b: D16 }>(),
	Flamework.createGuard<{ a: D17; b: D17 }>(),
	Flamework.createGuard<{ a: D18; b: D18 }>(),
	Flamework.createGuard<{ a: D19; b: D19 }>(),
	Flamework.createGuard<{ a: D20; b: D20 }>(),
	Flamework.createGuard<{ a: D21; b: D21 }>(),
	Flamework.createGuard<{ a: D22; b: D22 }>(),
	Flamework.createGuard<{ a: D23; b: D23 }>(),
	Flamework.createGuard<{ a: D24; b: D24 }>(),
	Flamework.createGuard<{ a: D25; b: D25 }>(),
	Flamework.createGuard<{ a: D26; b: D26 }>(),
	Flamework.createGuard<{ a: D27; b: D27 }>(),
	Flamework.createGuard<{ a: D28; b: D28 }>(),
	Flamework.createGuard<{ a: D29; b: D29 }>(),
	Flamework.createGuard<{ a: D30; b: D30 }>(),
	Flamework.createGuard<{ a: D31; b: D31 }>(),
	Flamework.createGuard<{ a: D32; b: D32 }>(),
	Flamework.createGuard<{ a: D33; b: D33 }>(),
	Flamework.createGuard<{ a: D34; b: D34 }>(),
	Flamework.createGuard<{ a: D35; b: D35 }>(),
	Flamework.createGuard<{ a: D36; b: D36 }>(),
	Flamework.createGuard<{ a: D37; b: D37 }>(),
	Flamework.createGuard<{ a: D38; b: D38 }>(),
	Flamework.createGuard<{ a: D39; b: D39 }>(),
];

/** A whole-statement send in the main chunk, after them. */
Events.w100.fire(text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text, text);
