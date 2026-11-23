// Transport only: send the clicked action and its fields to Rust, show the HTML it returns.
// No prices, totals, validation or state live here. Generic behaviours, all driven by markup
// Rust renders: <template id="screen-modal">, data-live forms, data-search inputs, data-copy.
const invoke = window.__TAURI__.core.invoke;
const view = document.getElementById("view");
const nav = document.getElementById("nav");
const overlay = document.getElementById("overlay");
const mount = document.getElementById("modalmount");

function note(text, alert) {
  const div = document.createElement("div");
  div.className = "help no-print";
  if (alert) div.setAttribute("role", "alert");
  div.textContent = text;
  view.prepend(div);
}

// Shows the modal Rust rendered in this response, or closes any open one.
function syncModal() {
  if (!mount || !overlay) return;
  const modal = view.querySelector("#screen-modal");
  mount.replaceChildren(...(modal ? [modal.content] : []));
  overlay.classList.toggle("show", Boolean(modal));
}

async function run(command, args) {
  try {
    view.innerHTML = await invoke(command, args);
    const screen = view.querySelector("#screen-nav");
    nav.replaceChildren(screen.content);
    document.getElementById("ttl").textContent = screen.dataset.title;
    document.getElementById("tsub").textContent = screen.dataset.subtitle;
    const actions = view.querySelector("#screen-actions");
    document.getElementById("top-actions").replaceChildren(...(actions ? [actions.content] : []));
    syncModal();
  } catch (error) {
    note(String(error), true);
  }
}

function fields(form, submitter) {
  return Object.fromEntries(new FormData(form, submitter));
}

async function copy(command) {
  try {
    const text = await invoke(command);
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(text);
    } else {
      const area = document.createElement("textarea");
      area.value = text;
      document.body.append(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    note("Copied ✓");
  } catch (error) {
    note("Copy failed: " + String(error), true);
  }
}

async function act(element, submitter) {
  const action = element.dataset.action;
  const data = element.dataset;
  if (data.copy) return copy(data.copy);
  if (action === "screen") {
    return run("screen", { tab: data.tab });
  }
  if (action === "print") {
    // Neither WKWebView nor Android's WebView implements window.print(): desktop asks Rust to
    // open the native dialog, Android uses the app's Kotlin print bridge. Both offer Save as PDF.
    return window.AlmAndroid
      ? window.AlmAndroid.print(data.title || document.title)
      : invoke("print_page");
  }
  if (action === "share") {
    const text = await invoke(data.command || "share_text");
    if (window.AlmAndroid) return window.AlmAndroid.share(text);
    if (navigator.share) return navigator.share({ text });
    return navigator.clipboard.writeText(text);
  }
  if (action === "share_image") {
    try {
      const path = await invoke(data.command, { id: data.id });
      if (window.AlmAndroid) return window.AlmAndroid.shareImage(path);
      return note("Saved " + path);
    } catch (error) {
      return note(String(error), true);
    }
  }
  if (action === "open_url") {
    try {
      const url = await invoke(data.command);
      if (window.AlmAndroid) return window.AlmAndroid.openUrl(url);
      return await invoke("open_url", { url });
    } catch (error) {
      return note(String(error), true);
    }
  }
  const values = element.tagName === "FORM" ? fields(element, submitter) : {};
  const args = data.payload ? { [data.payload]: values } : values;
  if (data.line) args.line = Number(data.line);
  if (data.id !== undefined) args.id = data.id;
  if (data.owner) args.owner = data.owner === "true";
  if (data.period) args.period = data.period;
  if (data.q !== undefined) args.q = data.q;
  return run(action, args);
}

// data-live forms: send the fields (plus the edited box's name) after typing pauses, then
// replace only the data-target fragment, keeping focus and caret in the box being typed in.
const timers = new Map();
async function live(form, input) {
  const values = fields(form);
  values._changed = input.name || "";
  const args = form.dataset.payload ? { [form.dataset.payload]: values } : values;
  try {
    const html = await invoke(form.dataset.live, args);
    const target = document.getElementById(form.dataset.target);
    if (!target) return;
    const focused = document.activeElement;
    const focusId = focused && focused.id;
    const caret = focused && focused.selectionStart;
    target.outerHTML = html;
    const again = focusId && document.getElementById(focusId);
    if (again) {
      again.focus();
      if (caret != null && again.setSelectionRange) again.setSelectionRange(caret, caret);
    }
  } catch (error) {
    note(String(error), true);
  }
}

// data-search inputs: ask Rust for matching options and list them under the box.
async function search(input) {
  const list = input.parentNode.querySelector(".autolist");
  if (!list) return;
  try {
    list.innerHTML = await invoke(input.dataset.search, { q: input.value, field: input.name });
    list.classList.toggle("show", list.children.length > 0);
  } catch (error) {
    note(String(error), true);
  }
}

// A picked option fills named fields of its form (data-fill is a query string), then
// optionally submits the form with the option's data-submit intent.
function pick(option) {
  const form = option.closest("form");
  const list = option.closest(".autolist");
  if (list) list.classList.remove("show");
  if (!form) return;
  for (const [name, value] of new URLSearchParams(option.dataset.fill)) {
    const box = form.elements.namedItem(name);
    if (box) box.value = value;
  }
  if (option.dataset.submit) {
    // A temporary hidden intent: act() reads the fields synchronously before it awaits.
    const intent = document.createElement("input");
    intent.type = "hidden";
    intent.name = "intent";
    intent.value = option.dataset.submit;
    form.append(intent);
    const sent = act(form);
    intent.remove();
    return sent;
  }
}

function onInput(event) {
  const input = event.target;
  const searching = Boolean(input.dataset && input.dataset.search);
  const key = searching ? input : input.closest && input.closest("form[data-live]");
  if (!key) return;
  clearTimeout(timers.get(key));
  timers.set(
    key,
    setTimeout(() => (searching ? search(input) : live(key, input)), searching ? 150 : 250),
  );
}

function onClick(event) {
  const option = event.target.closest("[data-fill]");
  if (option && option.dataset.fill != null) {
    if (event.preventDefault) event.preventDefault();
    return pick(option);
  }
  const target = event.target.closest("button[data-action],button[data-copy]");
  if (target) return act(target);
}

function onSubmit(event) {
  event.preventDefault();
  act(event.target, event.submitter);
}

document.getElementById("app").addEventListener("click", onClick);
for (const root of [view, overlay]) {
  if (!root) continue;
  root.addEventListener("submit", onSubmit);
  root.addEventListener("input", onInput);
}
if (overlay) {
  // A click on the dimmed backdrop presses the modal's own close button.
  overlay.addEventListener("mousedown", (event) => {
    if (event.target !== overlay) return;
    const close = overlay.querySelector(".modal .x[data-action]");
    if (close) act(close);
  });
}

// The device's local calendar date; Rust parses it and owns every date rule.
const now = new Date();
const pad = (n) => String(n).padStart(2, "0");
run("start", { today: `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}` });
