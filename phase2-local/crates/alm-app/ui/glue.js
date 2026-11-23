// Transport only: send the clicked action and its fields to Rust, show the HTML it returns.
// No prices, totals, validation or state live here.
const invoke = window.__TAURI__.core.invoke;
const view = document.getElementById("view");
let tab = "draft";

async function run(command, args) {
  try {
    view.innerHTML = await invoke(command, args);
  } catch (error) {
    const note = document.createElement("div");
    note.className = "help";
    note.textContent = String(error);
    view.prepend(note);
  }
}

function fields(form) {
  return form ? Object.fromEntries(new FormData(form).entries()) : {};
}

async function act(element) {
  const action = element.dataset.action;
  const data = element.dataset;
  if (action === "screen") {
    tab = data.tab;
    return run("screen", { tab });
  }
  if (action === "print") {
    // Neither WKWebView nor Android's WebView implements window.print(): desktop asks Rust to
    // open the native dialog, Android uses the app's Kotlin print bridge. Both offer Save as PDF.
    return window.AlmAndroid
      ? window.AlmAndroid.print(data.title || document.title)
      : invoke("print_page");
  }
  if (action === "share") {
    const text = await invoke("share_text");
    if (window.AlmAndroid) return window.AlmAndroid.share(text);
    if (navigator.share) return navigator.share({ text });
    return navigator.clipboard.writeText(text);
  }
  const args = { ...fields(element.tagName === "FORM" ? element : null) };
  if (data.line) args.line = Number(data.line);
  if (data.id) args.id = Number(data.id);
  if (data.owner) args.owner = data.owner === "true";
  return run(action, args);
}

view.addEventListener("click", (event) => {
  const target = event.target.closest("button[data-action]");
  if (target) act(target);
});
view.addEventListener("submit", (event) => {
  event.preventDefault();
  act(event.target.closest("form[data-action]"));
});
run("screen", { tab });
