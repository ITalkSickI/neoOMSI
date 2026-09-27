// openOMSI website: a small hash router that shows the overview, the download page and the
// Markdown files of docs/ (copied next to this page by .github/workflows/pages.yml).
const REPO = "turbo-devv/openOMSI";
const DOCS = [
  { file: "USER_GUIDE", title: "User guide", icon: "sports_esports" },
  { file: "BUILDING", title: "Building", icon: "build" },
  { file: "FORMATS", title: "Content formats", icon: "description" },
  { file: "ARCHITECTURE", title: "Architecture", icon: "account_tree" },
  { file: "RE_ROUTES", title: "Routes in Omsi.exe", icon: "alt_route" },
  { file: "PLUGINS", title: "Plugins", icon: "extension" },
  { file: "SERVER", title: "Dedicated server", icon: "dns" },
  { file: "VERSIONING", title: "Versioning & releases", icon: "new_releases" },
];
const PLATFORMS = [
  { key: "windows-x64", name: "Windows", icon: "desktop_windows", note: "64-bit, Windows 10 or newer" },
  { key: "macos-arm64", name: "macOS", icon: "laptop_mac", note: "Apple silicon, macOS 11 or newer" },
  { key: "linux-x64", name: "Linux", icon: "computer", note: "x86-64, Vulkan drivers" },
  { key: "server-linux-x64", name: "Dedicated server", icon: "dns", note: "Linux x86-64, no window" },
];

const view = document.getElementById("view");
const drawer = document.getElementById("drawer");
const scrim = document.getElementById("scrim");

document.getElementById("doc-links").innerHTML = DOCS.map(d =>
  `<a href="#/docs/${d.file}" data-route="/docs/${d.file}"><span class="material-icons">${d.icon}</span>${d.title}</a>`).join("");

function toggleDrawer(open) {
  drawer.classList.toggle("open", open);
  scrim.classList.toggle("open", open);
}
document.getElementById("menu-btn").onclick = () => toggleDrawer(!drawer.classList.contains("open"));
scrim.onclick = () => toggleDrawer(false);

let release = null;
async function latestRelease() {
  if (release) return release;
  try {
    const r = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`);
    release = r.ok ? await r.json() : { none: true };
  } catch { release = { none: true }; }
  return release;
}

function slug(text) {
  return text.toLowerCase().replace(/<[^>]+>/g, "").replace(/[^\w\- ]+/g, "").trim().replace(/\s+/g, "-");
}

function showTemplate(id) {
  view.innerHTML = "";
  view.appendChild(document.getElementById(id).content.cloneNode(true));
}

async function home() {
  showTemplate("home");
  const rel = await latestRelease();
  const chip = document.querySelector("#version-chip span:last-child");
  if (chip) chip.textContent = rel.tag_name ? `Latest: ${rel.tag_name.replace(/^v/, "")}` : "No release yet";
}

async function download() {
  showTemplate("download");
  const rel = await latestRelease();
  const grid = document.getElementById("dl-grid");
  const ver = document.getElementById("dl-version");
  if (!rel.tag_name) {
    ver.textContent = "No release has been published yet.";
    return;
  }
  const v = rel.tag_name.replace(/^v/, "");
  ver.innerHTML = `Latest version: <b>${v}</b> · ${new Date(rel.published_at).toLocaleDateString()}`;
  grid.innerHTML = PLATFORMS.map(p => {
    const a = (rel.assets || []).find(a => a.name.endsWith(`-${p.key}.zip`));
    const size = a ? ` · ${(a.size / 1048576).toFixed(0)} MB` : "";
    return `<div class="card elevation-1 dl-card"><span class="material-icons card-icon">${p.icon}</span>
      <h3>${p.name}</h3><p>${p.note}${size}</p>
      ${a ? `<a class="btn btn-contained" href="${a.browser_download_url}"><span class="material-icons">download</span>Download</a>`
          : `<p>Not in this release.</p>`}</div>`;
  }).join("");
}

async function doc(name, anchor) {
  const meta = DOCS.find(d => d.file === name);
  view.innerHTML = `<div class="content"><div class="doc"><div class="loading">Loading…</div></div></div>`;
  let md;
  try {
    const r = await fetch(`docs/${name}.md`);
    if (!r.ok) throw new Error(r.status);
    md = await r.text();
  } catch {
    view.querySelector(".doc").innerHTML = `<h1>Not found</h1><p>There is no document called <code>${name}</code>.</p>`;
    return;
  }
  const box = view.querySelector(".doc");
  box.innerHTML = `<div class="doc-toolbar"><a href="https://github.com/${REPO}/blob/main/docs/${name}.md">
    <span class="material-icons" style="font-size:18px">edit</span> Edit on GitHub</a></div>` + marked.parse(md);
  document.title = `${meta ? meta.title : name} · openOMSI`;
  // heading anchors
  box.querySelectorAll("h1, h2, h3, h4").forEach(h => {
    h.id = slug(h.textContent);
    if (h.tagName !== "H1") h.insertAdjacentHTML("beforeend", `<a class="anchor" href="#/docs/${name}#${h.id}">#</a>`);
  });
  // links between documents stay inside the site; other repository files go to GitHub
  box.querySelectorAll("a[href]").forEach(a => {
    const href = a.getAttribute("href");
    if (/^(https?:|mailto:|#)/.test(href)) return;
    const m = href.match(/^(?:\.\/)?([A-Z_]+)\.md(?:#(.*))?$/);
    if (m && DOCS.some(d => d.file === m[1])) {
      a.setAttribute("href", `#/docs/${m[1]}${m[2] ? "#" + m[2] : ""}`);
    } else {
      a.setAttribute("href", `https://github.com/${REPO}/blob/main/docs/${href}`);
    }
  });
  if (anchor) document.getElementById(anchor)?.scrollIntoView();
  else window.scrollTo(0, 0);
}

function route() {
  const hash = location.hash.replace(/^#/, "") || "/";
  const [path, anchor] = hash.split("#");
  document.querySelectorAll(".drawer a[data-route]").forEach(a => a.classList.toggle("active", a.dataset.route === path));
  toggleDrawer(false);
  document.title = "openOMSI";
  if (path.startsWith("/docs/")) return doc(path.slice(6), anchor);
  if (path === "/download") return download();
  window.scrollTo(0, 0);
  return home();
}
window.addEventListener("hashchange", route);
route();
