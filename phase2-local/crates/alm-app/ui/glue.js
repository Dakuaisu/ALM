// Transport only: send the clicked action and its fields to Rust, show the HTML it returns.
// No prices, totals, validation or state live here.
const invoke = window.__TAURI__.core.invoke;
const view = document.getElementById("view");
const nav = document.getElementById("nav");

async function run(command, args) {
  try {
    view.innerHTML = await invoke(command, args);
    const screen = view.querySelector("#screen-nav");
    nav.replaceChildren(screen.content);
    document.getElementById("ttl").textContent = screen.dataset.title;
    document.getElementById("tsub").textContent = screen.dataset.subtitle;
    const actions = view.querySelector("#screen-actions");
    document.getElementById("top-actions").replaceChildren(...(actions ? [actions.content] : []));
  } catch (error) {
    const note = document.createElement("div");
    note.className = "help";
    note.setAttribute("role", "alert");
    note.textContent = String(error);
    view.prepend(note);
  }
}

async function act(element, submitter) {
  const action = element.dataset.action;
  const data = element.dataset;
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
    const text = await invoke("share_text");
    if (window.AlmAndroid) return window.AlmAndroid.share(text);
    if (navigator.share) return navigator.share({ text });
    return navigator.clipboard.writeText(text);
  }
  const fields = element.tagName === "FORM" ? Object.fromEntries(new FormData(element, submitter)) : {};
  const args = data.payload ? { [data.payload]: fields } : fields;
  if (data.line) args.line = Number(data.line);
  if (data.id !== undefined) args.id = data.id;
  if (data.owner) args.owner = data.owner === "true";
  return run(action, args);
}

document.getElementById("app").addEventListener("click", (event) => {
  const target = event.target.closest("button[data-action]");
  if (target) act(target);
});
view.addEventListener("submit", (event) => {
  event.preventDefault();
  act(event.target, event.submitter);
});
run("screen", { tab: "bill" });
