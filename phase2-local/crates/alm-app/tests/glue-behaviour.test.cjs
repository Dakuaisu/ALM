// Generic glue behaviours driven by Rust markup: modal, live fragments, search/pick, clipboard.
const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { test } = require("node:test");
const { runInNewContext } = require("node:vm");

const glue = readFileSync(join(__dirname, "../ui/glue.js"), "utf8");
const tick = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function element(extra = {}) {
  const listeners = {};
  const classes = new Set();
  return {
    dataset: {},
    listeners,
    children: [],
    addEventListener(type, fn) {
      listeners[type] = fn;
    },
    classList: {
      add: (c) => classes.add(c),
      remove: (c) => classes.delete(c),
      toggle: (c, on) => (on ? classes.add(c) : classes.delete(c)),
      contains: (c) => classes.has(c),
    },
    replaceChildren(...kids) {
      this.children = kids;
    },
    prepend() {},
    querySelector: () => null,
    closest: () => null,
    ...extra,
  };
}

function harness(responses) {
  const calls = [];
  const view = element({
    querySelector: (selector) => (selector === "#screen-modal" ? null : { content: {}, dataset: {} }),
  });
  const ids = {
    app: element(),
    view,
    nav: element(),
    overlay: element(),
    modalmount: element(),
    "top-actions": element(),
    ttl: {},
    tsub: {},
  };
  const clipboard = [];
  const document = {
    activeElement: null,
    getElementById: (id) => ids[id],
    createElement: () => element({ remove() {}, setAttribute() {} }),
    body: element({ append() {} }),
  };
  class FormData {
    constructor(form) {
      this.entries = Object.entries(form.values || {});
    }
    [Symbol.iterator]() {
      return this.entries[Symbol.iterator]();
    }
  }
  const context = {
    document,
    FormData,
    URLSearchParams,
    setTimeout,
    clearTimeout,
    Map,
    navigator: { clipboard: { writeText: async (text) => clipboard.push(text) } },
    window: {
      __TAURI__: {
        core: {
          invoke: async (command, args) => {
            calls.push({ command, args: JSON.parse(JSON.stringify(args ?? null)) });
            const response = responses[command];
            return typeof response === "function" ? response(args) : response ?? "";
          },
        },
      },
    },
  };
  runInNewContext(glue, context);
  return { calls, ids, view, document, clipboard };
}

test("a rendered screen-modal template opens the overlay and a plain screen closes it", async () => {
  const h = harness({ screen: "<p>screen</p>" });
  const modal = { content: "MODAL" };
  h.view.querySelector = (selector) =>
    selector === "#screen-modal" ? modal : { content: {}, dataset: {} };
  await tick(5);
  assert.deepEqual(h.ids.modalmount.children, ["MODAL"]);
  assert.equal(h.ids.overlay.classList.contains("show"), true);
  h.view.querySelector = (selector) =>
    selector === "#screen-modal" ? null : { content: {}, dataset: {} };
  const button = { tagName: "BUTTON", dataset: { action: "screen", tab: "sales" } };
  await h.ids.app.listeners.click({ target: { closest: (s) => (s.includes("data-fill") ? null : button) } });
  assert.deepEqual(h.ids.modalmount.children, []);
  assert.equal(h.ids.overlay.classList.contains("show"), false);
});

test("data-live forms send their fields and the changed box, then replace only the target", async () => {
  const target = { outerHTML: "old" };
  const h = harness({ cash_live: '<div id="cash-system">new</div>' });
  h.document.getElementById = ((get) => (id) => (id === "cash-system" ? target : get(id)))(
    h.document.getElementById,
  );
  const form = {
    values: { cash: "25", date: "2025-04-30" },
    dataset: { live: "cash_live", target: "cash-system", payload: "terms" },
  };
  const input = { name: "cash", dataset: {}, closest: (s) => (s === "form[data-live]" ? form : null) };
  h.ids.view.listeners.input({ target: input });
  h.ids.view.listeners.input({ target: input });
  await tick(320);
  const live = h.calls.filter((c) => c.command === "cash_live");
  assert.equal(live.length, 1, "debounced to one request");
  assert.deepEqual(live[0].args, { terms: { cash: "25", date: "2025-04-30", _changed: "cash" } });
  assert.equal(target.outerHTML, '<div id="cash-system">new</div>');
});

test("data-search inputs list Rust options and a pick fills fields then submits its intent", async () => {
  const list = { innerHTML: "", children: [1], classList: element().classList };
  const h = harness({
    bill_customer_search: "<div data-fill>opt</div>",
    bill_header: "<p>bill</p>",
  });
  const input = {
    name: "customer_q",
    value: "syn",
    dataset: { search: "bill_customer_search" },
    parentNode: { querySelector: () => list },
  };
  h.ids.view.listeners.input({ target: input });
  await tick(200);
  assert.deepEqual(
    h.calls.find((c) => c.command === "bill_customer_search").args,
    { q: "syn", field: "customer_q" },
  );
  assert.equal(list.innerHTML, "<div data-fill>opt</div>");
  assert.equal(list.classList.contains("show"), true);

  const boxes = { customer: { value: "" }, customer_q: { value: "syn" } };
  const appended = [];
  const form = {
    tagName: "FORM",
    dataset: { action: "bill_header", payload: "header" },
    elements: { namedItem: (name) => boxes[name] },
    append: (node) => appended.push(node),
    get values() {
      const values = { customer: boxes.customer.value, customer_q: boxes.customer_q.value };
      for (const node of appended) if (!node.removed) values[node.name] = node.value;
      return values;
    },
  };
  const option = {
    dataset: { fill: "customer=batch1-customer&customer_q=Synthetic%20Customer", submit: "select" },
    closest: (s) => (s === "form" ? form : s === ".autolist" ? list : option),
  };
  h.document.createElement = () => ({ setAttribute() {}, remove() { this.removed = true; } });
  let prevented = false;
  await h.ids.app.listeners.pointerdown({ target: { closest: () => option }, preventDefault() { prevented = true; } });
  assert.equal(prevented, true, "the box keeps focus");
  assert.equal(h.ids.app.listeners.click({ target: { closest: () => option } }), undefined, "the click after a pick is ignored");
  assert.equal(boxes.customer.value, "batch1-customer");
  assert.equal(boxes.customer_q.value, "Synthetic Customer");
  const submit = h.calls.find((c) => c.command === "bill_header");
  assert.deepEqual(submit.args, {
    header: { customer: "batch1-customer", customer_q: "Synthetic Customer", intent: "select" },
  });
  assert.equal(appended[0].removed, true, "temporary intent removed");
});

test("data-copy writes the Rust-provided text to the clipboard", async () => {
  const h = harness({ godown_copy_text: "*Godown Stock* (30-Apr-2025)" });
  const button = { tagName: "BUTTON", dataset: { copy: "godown_copy_text" } };
  await h.ids.app.listeners.click({
    target: { closest: (s) => (s.includes("data-fill") ? null : button) },
  });
  await tick(5);
  assert.deepEqual(h.clipboard, ["*Godown Stock* (30-Apr-2025)"]);
  assert.equal(h.calls.at(-1).command, "godown_copy_text");
});

test("a search box losing focus closes its list", async () => {
  const list = { classList: element().classList };
  list.classList.add("show");
  const h = harness({});
  h.ids.app.listeners.focusout({
    target: { dataset: { search: "x" }, parentNode: { querySelector: () => list } },
  });
  await tick(200);
  assert.equal(list.classList.contains("show"), false);
});
