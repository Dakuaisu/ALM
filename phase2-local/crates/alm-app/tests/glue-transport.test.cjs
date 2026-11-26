const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { test } = require("node:test");
const { runInNewContext } = require("node:vm");

const glue = readFileSync(join(__dirname, "../ui/glue.js"), "utf8");

function harness(invoke, { session = null } = {}) {
  let click;
  let resolveReady;
  let html = "";
  const ready = new Promise((resolve) => { resolveReady = resolve; });
  const notes = [];
  const retryDelays = [];
  const nav = { content: {}, dataset: { title: "Bills", subtitle: "Drafts" } };
  const actions = { content: {} };
  const templates = {
    "#screen-nav": nav,
    "#screen-actions": actions,
    "#screen-session": session && { dataset: session },
  };
  const view = {
    addEventListener() {},
    querySelector: (selector) => templates[selector] ?? null,
    prepend(node) { notes.push(node); resolveReady(); },
    set innerHTML(value) { html = value; resolveReady(); },
    get innerHTML() { return html; },
  };
  const container = () => ({ replaceChildren(...children) { this.children = children; } });
  const elements = {
    app: { addEventListener: (type, listener) => { if (type === "click") click = listener; } },
    view,
    nav: container(),
    "top-actions": container(),
    ttl: {},
    tsub: {},
    bizline: { textContent: "unchanged account" },
    synctxt: { textContent: "unchanged authority" },
    syncdot: { className: "unchanged dot" },
  };
  const document = {
    getElementById: (id) => elements[id] ?? null,
    createElement: (tag) => {
      assert.equal(tag, "div");
      return {
        attributes: {},
        setAttribute(name, value) { this.attributes[name] = value; },
      };
    },
  };
  runInNewContext(glue, {
    document,
    setTimeout: (fn, delay) => { retryDelays.push(delay); return setImmediate(fn); },
    window: { __TAURI__: { core: { invoke } } },
  });
  return { click: (event) => click(event), ready, notes, retryDelays, elements, nav, actions };
}

function button(action, id) {
  return { target: { closest: () => ({ tagName: "BUTTON", dataset: { action, id } }) } };
}

for (const action of ["draft_open", "sale_confirm", "sale_cancel", "sale_restore"]) {
  test(`${action} sends the exact decimal ID in its IPC payload`, async () => {
    const requests = [];
    const session = {
      account: "Manager · Example",
      authority: "Built-in simulation",
      dot: "sync-ok",
    };
    const h = harness(async (command, args) => {
      requests.push({ command, args: JSON.parse(JSON.stringify(args)) });
      return `<p>${command}</p>`;
    }, { session });
    await h.ready;
    assert.equal(requests[0].command, "start");
    assert.equal(h.elements.view.innerHTML, "<p>start</p>");
    assert.equal(h.elements.bizline.textContent, session.account);
    assert.equal(h.elements.synctxt.textContent, session.authority);
    assert.equal(h.elements.syncdot.className, session.dot);

    const ids = ["9007199254740993", "18446744073709551615"];
    for (const id of ids) {
      session.account = `Account ${id}`;
      session.authority = `Authority ${id}`;
      session.dot = `dot-${id}`;
      await h.click(button(action, id));
      assert.equal(h.elements.view.innerHTML, `<p>${action}</p>`);
      assert.equal(h.elements.bizline.textContent, session.account);
      assert.equal(h.elements.synctxt.textContent, session.authority);
      assert.equal(h.elements.syncdot.className, session.dot);
    }

    assert.deepEqual(requests.slice(1), ids.map((id) => ({ command: action, args: { id } })));
    assert.deepEqual(h.notes, []);
  });
}

test("start is retried while the app state is not there yet, then shows the screen", async () => {
  const calls = [];
  const h = harness(async (command) => {
    calls.push(command);
    if (calls.length < 3) throw "state not managed for field `state` on command `start`";
    return "<p>bill</p>";
  });
  await h.ready;
  assert.deepEqual(calls, ["start", "start", "start"]);
  assert.deepEqual(h.retryDelays, [250, 250]);
  assert.equal(h.elements.view.innerHTML, "<p>bill</p>");
  assert.deepEqual(h.elements.nav.children, [h.nav.content]);
  assert.deepEqual(h.elements["top-actions"].children, [h.actions.content]);
  assert.equal(h.elements.ttl.textContent, "Bills");
  assert.equal(h.elements.tsub.textContent, "Drafts");
  assert.equal(h.elements.bizline.textContent, "unchanged account");
  assert.equal(h.elements.synctxt.textContent, "unchanged authority");
  assert.equal(h.elements.syncdot.className, "unchanged dot");
  assert.deepEqual(h.notes, []);
});

test("a non-retryable startup failure is displayed as an alert", async () => {
  const calls = [];
  const h = harness(async (command) => {
    calls.push(command);
    throw new Error("startup refused");
  });
  await h.ready;
  assert.deepEqual(calls, ["start"]);
  assert.deepEqual(h.retryDelays, []);
  assert.equal(h.elements.view.innerHTML, "");
  assert.equal(h.notes.length, 1);
  assert.equal(h.notes[0].textContent, "Error: startup refused");
  assert.equal(h.notes[0].attributes.role, "alert");
});

test("an action click waits for its IPC response to render", async () => {
  let resolveAction;
  const response = new Promise((resolve) => { resolveAction = resolve; });
  const h = harness(async (command) => command === "start" ? "<p>home</p>" : response);
  await h.ready;
  let settled = false;
  const pending = h.click(button("draft_open", "9007199254740993"));
  pending.then(() => { settled = true; });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(settled, false);
  assert.equal(h.elements.view.innerHTML, "<p>home</p>");
  resolveAction("<p>draft</p>");
  await pending;
  assert.equal(settled, true);
  assert.equal(h.elements.view.innerHTML, "<p>draft</p>");
  assert.deepEqual(h.notes, []);
});

test("a rejected action displays an alert without replacing the screen", async () => {
  const h = harness(async (command) => {
    if (command === "start") return "<p>home</p>";
    throw new Error("action refused");
  });
  await h.ready;
  await h.click(button("draft_open", "9007199254740993"));
  assert.equal(h.elements.view.innerHTML, "<p>home</p>");
  assert.equal(h.notes.length, 1);
  assert.equal(h.notes[0].textContent, "Error: action refused");
  assert.equal(h.notes[0].attributes.role, "alert");
});
