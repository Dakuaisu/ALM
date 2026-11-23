"use strict";

const assert = require("node:assert/strict");
const { createHash } = require("node:crypto");
const { createReadStream } = require("node:fs");
const { createInterface } = require("node:readline");
const { before, test } = require("node:test");
const { Script, createContext } = require("node:vm");

// PHASE1_CASES.md:621. Read source as text only, stopping after the last helper.
const filename = "/Users/apple/ALM/ALM_Business_Manager.html";
const pins = [
  ["godownMaps", 10850, 10905, "492f7ce9164b91dc5aa6a031cb447cc9daa643454d6c42bcacbf7b1aa9cee69b"],
  ["stockBefore", 10907, 10914, "b025a9ca221be598212534f35a9f5053cce328089a0ab0c9c68e51b220da0ed9"],
  ["movementOn", 10916, 10924, "dcfdb3fd6817919d886be2e7c8be853b432c93c21d40c1525365d62f9b9674e8"],
];
const firstLine = pins[0][1];
const lastLine = pins.at(-1)[2];
let frozen;

before(async () => {
  const input = createReadStream(filename);
  const lines = [];
  let lineNumber = 0;
  try {
    for await (const line of createInterface({ input, crlfDelay: Infinity })) {
      lineNumber += 1;
      if (lineNumber >= firstLine) lines.push(line);
      if (lineNumber === lastLine) break;
    }
  } finally {
    input.destroy();
  }

  frozen = pins.map(([name, start, end, expectedHash]) => {
    const body = lines.slice(start - firstLine, end - firstLine + 1).join("\n");
    const sha256 = createHash("sha256").update(body).digest("hex");
    assert.equal(sha256, expectedHash, `frozen ${name} at ${filename}:${start}`);
    console.log(`l12_02 frozen extraction: ${name} ${filename}:${start}-${end} sha256=${sha256}`);
    return new Script(body, { filename, lineOffset: start - 1 });
  });
});

const invoke = new Script(`
  const { db: DB, item, dates } = JSON.parse(fixture);
  const keyOfName = (name) => name;
  const before = JSON.stringify(DB);
  const rows = dates.map(date => {
    const gm = godownMaps(date);
    const opening = stockBefore(item, date, gm);
    const { recv: received, disp: dispatched } = movementOn(item, date, gm);
    return { date, opening, received, dispatched, closing: opening + received - dispatched };
  });
  JSON.stringify({ rows, before, after: JSON.stringify(DB) });
`);

const item = "synthetic-packet";
const poDate = "2025-02-10";
const receiptDate = "2025-02-12";
const dates = ["2025-02-09", poDate, receiptDate, "2025-02-13"];

// Each expected tuple is [opening, received, closing] for the corresponding selected date.
for (const { name, po, receipts, expected } of [
  {
    name: "PO-date fallback (not an actual historical receipt date) and later dated log",
    po: { date: poDate }, receipts: [{ date: receiptDate, qty: 2 }],
    expected: [[0, 0, 0], [0, 3, 3], [3, 2, 5], [5, 0, 5]],
  },
  {
    name: "fully logged: no PO-date fallback",
    po: { date: poDate }, receipts: [{ date: receiptDate, qty: 5 }],
    expected: [[0, 0, 0], [0, 0, 0], [0, 5, 5], [5, 0, 5]],
  },
  {
    name: "undated log goes into opening, not the PO day's Received",
    po: { date: poDate }, receipts: [{ qty: 2 }],
    expected: [[2, 0, 2], [2, 3, 5], [5, 0, 5], [5, 0, 5]],
  },
  {
    name: "missing PO date puts the fallback into opening",
    po: {}, receipts: [{ date: receiptDate, qty: 2 }],
    expected: [[3, 0, 3], [3, 0, 3], [3, 2, 5], [5, 0, 5]],
  },
]) {
  test(`l12_02 frozen godown: ${name}`, () => {
    // Given only a synthetic PO line with cumulative received 5 and no other movements.
    const db = {
      purchases: [], sales: [], stockLog: [], stock: { [item]: { opening: 0 } },
      purchaseOrders: [{ ...po, items: [{ item, received: 5, receipts }] }],
    };
    const context = createContext({ fixture: JSON.stringify({ db, item, dates }) }, {
      codeGeneration: { strings: false, wasm: false },
    });
    for (const script of frozen) script.runInContext(context, { timeout: 1000 });

    // When the frozen helpers calculate the register for each selected date.
    const observed = JSON.parse(invoke.runInContext(context, { timeout: 1000 }));

    // Then receipts land in separate buckets, closing never double-counts, and DB is unchanged.
    assert.equal(observed.before, JSON.stringify(db), "VM started with only the synthetic DB");
    assert.equal(observed.after, observed.before, "register projection must leave the synthetic DB unchanged");
    assert.deepEqual(observed.rows, expected.map(([opening, received, closing], i) => ({
      date: dates[i], opening, received, dispatched: 0, closing,
    })));
    console.log(`l12_02 ${name}: ${JSON.stringify(observed.rows)}`);
  });
}
