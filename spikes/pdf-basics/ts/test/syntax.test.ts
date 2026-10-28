// SPEC §1 serialization rules and §3 lexer details.
import { describe, expect, test } from "vitest";
import {
  ContentBuilder,
  dict,
  fixed,
  formatFixed,
  formatReal,
  hexStr,
  int,
  name,
  parseObject,
  real,
  serialize,
  str,
  unpredict,
} from "../src/index.js";
import { bytesOf, expectPdfError, latin1 } from "./helpers.js";

const ser = (o: Parameters<typeof serialize>[0]) => latin1(serialize(o));
const parse = (s: string) => parseObject(bytesOf(s));

describe("number formatting", () => {
  test.each([
    [0.5, "0.5"],
    [1, "1"],
    [-0.25, "-0.25"],
    [-0, "0"],
    [-1e-9, "0"],
    [1e21, "1000000000000000000000"],
    [1 / 3, "0.333333"],
    [0.1 + 0.2, "0.3"],
    [612, "612"],
    [2.5e-6, "0.000003"], // 2.5e-6 is slightly above the tie in binary
    [0.0000005, "0"], // 5e-7 is slightly below the tie in binary → rounds down
  ])("real %d → %s", (x, s) => expect(formatReal(x)).toBe(s));

  test("formatFixed matches Python's round-half-even on the exact binary value", () => {
    // Exact binary ties: Python rounds to even, toFixed rounds up.
    expect(formatFixed(0.125, 2)).toBe("0.12");
    expect((0.125).toFixed(2)).toBe("0.13");
    expect(formatFixed(0.375, 2)).toBe("0.38");
    expect(formatFixed(2.5, 0)).toBe("2");
    expect(formatFixed(3.5, 0)).toBe("4");
    expect(formatFixed(-0.125, 2)).toBe("-0.12");
    // Non-ties are decided by the exact binary value (1.005 is 1.00499999…).
    expect(formatFixed(1.005, 2)).toBe("1.00");
    expect(formatFixed(500, 3)).toBe("500.000");
    expect(formatFixed(-0, 3)).toBe("-0.000"); // like Python
    expect(formatFixed(-0.0001, 3)).toBe("-0.000");
    expect(formatFixed(Math.cos(Math.PI / 6), 6)).toBe("0.866025");
  });

  test("non-finite numbers are rejected", () => {
    expectPdfError(() => serialize(real(Number.NaN)), "Invalid");
    expectPdfError(() => formatReal(Number.POSITIVE_INFINITY), "Invalid");
  });
});

describe("serialization", () => {
  test("names escape non-regular bytes, #, and delimiters with uppercase #xx", () => {
    expect(ser(name("A B"))).toBe("/A#20B");
    expect(ser(name("a#b"))).toBe("/a#23b");
    expect(ser(name("()<>[]{}/%"))).toBe("/#28#29#3C#3E#5B#5D#7B#7D#2F#25");
    expect(ser(name("\x00\xff~!"))).toBe("/#00#FF~!");
  });
  test("literal strings escape \\ ( ) and CR only", () => {
    expect(ser(str("a(b)c\\d\re\nf\t"))).toBe("(a\\(b\\)c\\\\d\\re\nf\t)");
  });
  test("hex strings are uppercase", () => expect(ser(hexStr("\x01\xab"))).toBe("<01AB>"));
  test("dicts and arrays", () => {
    expect(ser(dict([["K", int(1)], ["K2", parse("[1 2.5 /n (s) <AB> true null 3 0 R]")]]))).toBe(
      "<< /K 1 /K2 [1 2.5 /n (s) <AB> true null 3 0 R] >>",
    );
    expect(ser(dict())).toBe("<< >>");
  });
});

describe("lexer / parser", () => {
  test("literal string escapes, octal, continuation, balanced parens", () => {
    const s = parse("(a\\n\\r\\t\\b\\f\\(\\)\\\\\\101\\7\\0612\\\nb\\\r\nc (nested (deep)) \\q)");
    expect(s.kind === "string" && latin1(s.bytes)).toBe("a\n\r\t\b\f()\\A\x07\x312bc (nested (deep)) q");
  });
  test("unescaped CR / CRLF in a literal string read as LF", () => {
    const s = parse("(a\rb\r\nc)");
    expect(s.kind === "string" && latin1(s.bytes)).toBe("a\nb\nc");
  });
  test("hex strings with whitespace and odd length", () => {
    const s = parse("< 4 1\n42 4 >");
    expect(s.kind === "string" && s.hex && latin1(s.bytes)).toBe("AB@");
  });
  test("comments and every whitespace kind", () => {
    const o = parse("%c1\n<<\x00/A\t1\x0c/B\r\n% inner comment\n2.5\x20/C [ 1 2 R ] >>");
    expect(ser(o)).toBe("<< /A 1 /B 2.5 /C [1 2 R] >>");
  });
  test("name #xx decoding; invalid escapes left literal", () => {
    const o = parse("/A#20B#2");
    expect(o).toEqual(name("A B#2"));
  });
  test("numbers: +5, -.5, 4., and refs vs plain ints", () => {
    expect(ser(parse("[+5 -.5 4. 1 2 3 0 R 7]"))).toBe("[5 -0.5 4 1 2 3 0 R 7]");
  });
  test("integers beyond the safe range degrade to a real", () => {
    expect(parse("99999999999999999999")).toEqual({ kind: "real", value: 1e20 });
    expect(parse("-9007199254740993")).toEqual({ kind: "real", value: -9007199254740992 });
    expect(parse("9007199254740991")).toEqual({ kind: "int", value: 9007199254740991 });
  });
  test("errors are typed", () => {
    expectPdfError(() => parse("(unterminated"), "UnexpectedEof");
    expectPdfError(() => parse("<zz>"), "Syntax");
    expectPdfError(() => parse("[1 2"), "UnexpectedEof");
    expectPdfError(() => parse("<< 1 2 >>"), "Syntax");
    expectPdfError(() => parse(""), "UnexpectedEof");
  });
});

describe("filters", () => {
  test("PNG predictors (Up, Sub, Average, Paeth)", () => {
    // 2 columns, 1 byte per pixel: rows encode [10,20] [11,22] [12,24] [13,26].
    const enc = Uint8Array.from([0, 10, 20, 2, 1, 2, 1, 12, 12, 4, 1, 2]);
    expect([...unpredict(enc, { predictor: 12, colors: 1, bitsPerComponent: 8, columns: 2 })]).toEqual([
      10, 20, 11, 22, 12, 24, 13, 26,
    ]);
    const avg = Uint8Array.from([3, 10, 20, 3, 6, 6]);
    // row0 avg with prev=0: [10, 20+5=25]; row1: [6+5=11, 6+((11+25)>>1)=24]
    expect([...unpredict(avg, { predictor: 13, colors: 1, bitsPerComponent: 8, columns: 2 })]).toEqual([10, 25, 11, 24]);
  });
});

describe("content builder", () => {
  test("operators, inline grouping, explicit precision", () => {
    const b = new ContentBuilder()
      .comment("hi")
      .q()
      .cm(1, 0, 0, 1, fixed(0.5, 2), -0.25)
      .inline((x) => x.J(1).j(2).w(0.5))
      .d([3, 1.5], 0)
      .m(0, 0).l(10, 10).h().S()
      .BT().Tf("F 1", 12).TL(14).Td(1, 2).Tj("a(b)").quote("c").ET()
      .Q();
    expect(b.toString()).toBe(
      "% hi\nq\n1 0 0 1 0.50 -0.25 cm\n1 J 2 j 0.5 w\n[3 1.5] 0 d\n0 0 m\n10 10 l\nh\nS\nBT\n/F#201 12 Tf\n14 TL\n1 2 Td\n(a\\(b\\)) Tj\n(c) '\nET\nQ",
    );
  });
  test("circle helper uses 3-decimal Python-compatible formatting", () => {
    expect(new ContentBuilder().circle(430, 660, 70).toString()).toBe(
      [
        "500.000 660.000 m",
        "500.000 698.660 468.660 730.000 430.000 730.000 c",
        "391.340 730.000 360.000 698.660 360.000 660.000 c",
        "360.000 621.340 391.340 590.000 430.000 590.000 c",
        "468.660 590.000 500.000 621.340 500.000 660.000 c",
        "h",
      ].join("\n"),
    );
  });
});
