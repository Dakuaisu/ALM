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

test("start is retried while the app state is not there yet, then shows the screen", async () => {
  let calls = 0;
  let shown = "";
  const view = {
    addEventListener() {},
    querySelector: () => ({ content: {}, dataset: {} }),
    set innerHTML(html) { shown = html; },
  };
  const elements = {
    app: { addEventListener() {} },
    view,
    nav: { replaceChildren() {} },
    "top-actions": { replaceChildren() {} },
    ttl: {},
    tsub: {},
  };
  runInNewContext(glue, {
    document: { getElementById: (id) => elements[id] },
    setTimeout: (fn) => setImmediate(fn),
    window: { __TAURI__: { core: { invoke: async (command) => {
      assert.equal(command, "start");
      calls += 1;
      if (calls < 3) throw "state not managed for field `state` on command `start`";
      return "<p>bill</p>";
    } } } },
  });
  await new Promise((resolve) => setTimeout(resolve, 50));
  assert.equal(calls, 3);
  assert.equal(shown, "<p>bill</p>");
});
