"use strict";

const assert = require("node:assert/strict");
const { createHash } = require("node:crypto");
const { readFileSync } = require("node:fs");
const { resolve } = require("node:path");
const { test } = require("node:test");
const { Script, createContext } = require("node:vm");

// Frozen legacy function only. No HTML script block is loaded or run.
const filename = resolve(__dirname, "../../ALM_Business_Manager.html");
const html = readFileSync(filename, "utf8").replace(/\r\n/g, "\n");
const matches = [...html.matchAll(/^function fifoCostOf\(item, qty, o\) \{\n[\s\S]*?^\}/gm)];
assert.equal(matches.length, 1, "expected exactly one frozen fifoCostOf definition");
const [body] = matches[0];
const line = html.slice(0, matches[0].index).split("\n").length;
const sha256 = createHash("sha256").update(body).digest("hex");
assert.equal(sha256, "35822c6acfc4f1ebe51995856fd00ebc3921bd01f9b3f7c1c93910f0306f635b");
const fifo = new Script(body, { filename, lineOffset: line - 1 });
console.log(`l11_01 frozen extraction: ${filename}:${line} sha256=${sha256}`);

const invoke = new Script(`
  const { product, lots, sold } = JSON.parse(fixture);
  const prodByItem = () => product;
  const lotsFor = () => lots;
  const keyOfName = (item) => item;
  JSON.stringify(fifoCostOf("synthetic", 8, {
    qm: { sold: { synthetic: sold }, adj: {} }
  }));
`);
const lots = [
  { date: "2025-01-01", qty: 2, rate: 5 },
  { date: "2025-01-02", qty: 3, rate: 11 },
];
const available = [{ ...lots[0], qty: 1 }, lots[1]];

for (const { name, fixture, expected } of [
  {
    name: "nonzero buy rate overrides last lot",
    fixture: { product: { buyRate: 31 }, lots, sold: 1 },
    expected: {
      unit: 20.25, total: 162,
      lots: [...available, { date: "", qty: 4, rate: 31, short: true }],
    },
  },
  {
    name: "zero buy rate uses last lot",
    fixture: { product: { buyRate: 0 }, lots, sold: 1 },
    expected: {
      unit: 10.25, total: 82,
      lots: [...available, { date: "", qty: 4, rate: 11, short: true }],
    },
  },
  {
    name: "missing buy rate uses last lot even if exhausted",
    fixture: { product: {}, lots, sold: 5 },
    expected: { unit: 11, total: 88, lots: [{ date: "", qty: 8, rate: 11, short: true }] },
  },
  {
    name: "no lot and no buy rate values shortage at zero",
    fixture: { product: {}, lots: [], sold: 0 },
    expected: { unit: 0, total: 0, lots: [{ date: "", qty: 8, rate: 0, short: true }] },
  },
]) {
  test(`l11_01 frozen fifoCostOf: ${name}`, () => {
    // Given a fresh in-memory copy of the synthetic product, lots and sold quantities.
    const context = createContext({ fixture: JSON.stringify(fixture) }, {
      codeGeneration: { strings: false, wasm: false },
    });
    fifo.runInContext(context, { timeout: 1000 });

    // When the extracted function values eight requested units.
    const actual = JSON.parse(invoke.runInContext(context, { timeout: 1000 }));

    // Then the complete valuation, including the shortage entry, matches.
    assert.deepEqual(actual, expected);
    console.log(`l11_01 ${name}: ${JSON.stringify(actual)}`);
  });
}
