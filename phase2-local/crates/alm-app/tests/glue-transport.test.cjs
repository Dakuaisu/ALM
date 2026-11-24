const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { test } = require("node:test");
const { runInNewContext } = require("node:vm");

const glue = readFileSync(join(__dirname, "../ui/glue.js"), "utf8");

for (const action of ["draft_open", "sale_confirm", "sale_cancel", "sale_restore"]) {
  test(`${action} sends the exact decimal ID in its IPC payload`, () => {
    const requests = [];
    const view = {
      addEventListener() {},
      querySelector: () => ({ content: {}, dataset: {} }),
    };
    let click;
    const elements = {
      app: { addEventListener: (type, listener) => { if (type === "click") click = listener; } },
      view,
      nav: { replaceChildren() {} },
      "top-actions": { replaceChildren() {} },
      ttl: {},
      tsub: {},
    };
    runInNewContext(glue, {
      document: { getElementById: (id) => elements[id] },
      window: { __TAURI__: { core: { invoke: async (command, args) => {
        requests.push({ command, args: JSON.parse(JSON.stringify(args)) });
        return "";
      } } } },
    });

    const ids = ["9007199254740993", "18446744073709551615"];
    for (const id of ids) {
      click({ target: { closest: () => ({ tagName: "BUTTON", dataset: { action, id } }) } });
    }

    assert.deepEqual(requests.slice(1), ids.map((id) => ({ command: action, args: { id } })));
  });
}
