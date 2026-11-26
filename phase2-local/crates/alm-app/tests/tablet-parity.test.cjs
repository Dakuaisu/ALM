const assert = require("node:assert/strict");
const { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { join } = require("node:path");
const { spawnSync } = require("node:child_process");
const { test } = require("node:test");
const { pathToFileURL } = require("node:url");

const browser = [
  process.env.ALM_TEST_CHROME,
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
  "/usr/bin/chromium-browser",
  process.env.PROGRAMFILES && join(process.env.PROGRAMFILES, "Google/Chrome/Application/chrome.exe"),
].find((path) => path && existsSync(path));
const ui = join(__dirname, "../ui");
const fixture = `
  <template id="screen-nav" data-title="Synthetic screen" data-subtitle="Tablet test">
    <button id="nav-control" class="navitem" type="button">Navigate</button>
  </template>
  <template id="screen-actions"><button class="btn" type="button">Synthetic action</button></template>
  <div class="row">
    <button id="small-button" class="btn sm" type="button">Edit</button>
    <button id="close-button" class="x" type="button">×</button>
    <button id="link-button" class="list-link" type="button">Open</button>
    <details><summary id="summary" class="btn sm">More</summary>Details</details>
    <input id="text-input" class="inp" aria-label="Synthetic text">
    <select id="select" class="inp" aria-label="Synthetic choice"><option>One</option></select>
    <textarea id="textarea" class="inp" aria-label="Synthetic notes"></textarea>
    <label><input id="checkbox" type="checkbox"> Check</label>
    <label><input id="radio" type="radio" name="synthetic"> Radio</label>
  </div>
  <div class="autobox" style="width:200px"><div class="autolist show" role="listbox">
    <div id="option" role="option" tabindex="-1" data-fill="synthetic=1">Synthetic option</div>
  </div></div>
  <div class="tbl-wrap" id="wrapped"><table><tbody><tr>
    <td style="min-width:1400px">Synthetic wide table</td>
  </tr></tbody></table></div>
  <table id="bare"><tbody><tr><td style="min-width:1400px">Synthetic wide table</td></tr></tbody></table>
`;

function inspect(width, height) {
  const dir = mkdtempSync(join(tmpdir(), "alm-tablet-test-"));
  try {
    const styles = ["alm.css", "app.css"].map((name) => readFileSync(join(ui, name), "utf8")).join("\n");
    const glue = readFileSync(join(ui, "glue.js"), "utf8");
    const shell = readFileSync(join(ui, "index.html"), "utf8")
      .replace(/<link rel="stylesheet" href="(?:alm|app)\.css">/g, "")
      .replace("</head>", `<style>${styles}</style></head>`)
      .replace('<script src="glue.js"></script>', `
        <script>window.__TAURI__ = {core: {invoke: async () => ${JSON.stringify(fixture)}}};</script>
        <script>${glue}</script>
        <pre id="results"></pre>
        <script>
          setTimeout(() => {
            const ids = ["nav-control", "small-button", "close-button", "link-button", "summary",
              "text-input", "select", "textarea", "checkbox", "radio", "option"];
            const controls = Object.fromEntries(ids.map((id) => {
              const element = document.getElementById(id);
              const box = element.getBoundingClientRect();
              element.focus();
              const style = getComputedStyle(element);
              return [id, {width: box.width, height: box.height,
                focus: document.activeElement === element && style.outlineStyle !== "none" && parseFloat(style.outlineWidth) >= 2}];
            }));
            const scroll = Object.fromEntries(["wrapped", "bare"].map((id) => {
              const element = document.getElementById(id);
              element.scrollLeft = 200;
              return [id, {client: element.clientWidth, content: element.scrollWidth, left: element.scrollLeft}];
            }));
            const view = document.getElementById("view");
            document.getElementById("results").textContent = btoa(JSON.stringify({
              viewport: [innerWidth, innerHeight], controls, scroll,
              view: {client: view.clientWidth, content: view.scrollWidth}
            }));
          }, 100);
        </script>`);
    const file = join(dir, "fixture.html");
    writeFileSync(file, shell);
    const result = spawnSync(browser, [
      "--headless=new", "--disable-gpu", "--disable-background-networking", "--disable-sync",
      "--no-first-run", "--no-default-browser-check", "--disable-component-update",
      "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost", "--touch-events=enabled",
      `--user-data-dir=${join(dir, "profile")}`, `--window-size=${width},${height}`,
      "--virtual-time-budget=1000", "--dump-dom", pathToFileURL(file).href,
    ], { encoding: "utf8", timeout: 30000, maxBuffer: 4 * 1024 * 1024 });
    assert.equal(result.status, 0, result.stderr || String(result.error));
    const encoded = result.stdout.match(/<pre id="results">([^<]+)<\/pre>/);
    assert.ok(encoded, "browser did not return the fixture measurements");
    return JSON.parse(Buffer.from(encoded[1], "base64").toString("utf8"));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

for (const [orientation, width, height] of [["portrait", 800, 1280], ["landscape", 1280, 800]]) {
  test(`tablet ${orientation}: touch targets, focus rings, and table scrolling`, { skip: !browser && "Chrome/Chromium is not installed" }, () => {
    const result = inspect(width, height);
    if (process.env.ALM_TABLET_MEASURE) console.log(`${orientation}: ${JSON.stringify(result)}`);
    assert.equal(result.viewport[0], width);
    assert.equal(result.viewport[1] > result.viewport[0], orientation === "portrait");
    for (const [id, control] of Object.entries(result.controls)) {
      assert.ok(control.width >= 48 && control.height >= 48, `${id}: ${control.width}x${control.height}`);
      assert.equal(control.focus, true, `${id}: keyboard focus is not visible`);
    }
    for (const [id, table] of Object.entries(result.scroll)) {
      assert.ok(table.content > table.client && table.left > 0, `${id}: table cannot scroll horizontally`);
    }
    assert.ok(result.view.content <= result.view.client, "wide tables must not widen the main view");
  });
}
