// The stats page (GET /): one HTML page drawn from stats() (index.js), with no scripts from elsewhere.
// Copyright the OpenSGolf contributors. GPL-2.0-or-later.

const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
const h1 = (n) => (n >= 100 ? Math.round(n).toLocaleString("en-GB") : (Math.round(n * 10) / 10).toLocaleString("en-GB"));
const hrs = (n) => `${h1(n)} h`;

/** Horizontal bars: rows of {label, value, note?}; one hue (one measure). */
function bars(rows, fmt = hrs) {
  const max = Math.max(1e-9, ...rows.map((r) => r.value));
  return `<div class="bars" role="table">${rows.map((r) => `
    <div class="bar" role="row" title="${esc(r.label)}: ${esc(fmt(r.value))}${r.note ? " · " + esc(r.note) : ""}">
      <span class="bl" role="cell">${esc(r.label)}</span>
      <span class="bt" role="cell"><span class="bf" style="width:${(100 * r.value / max).toFixed(2)}%"></span></span>
      <span class="bv" role="cell">${esc(fmt(r.value))}${r.note ? `<small> ${esc(r.note)}</small>` : ""}</span>
    </div>`).join("")}</div>`;
}

/** The daily chart: stacked columns, GoatCounter (older builds) then this counter, with a per-day tooltip. */
function daily(days) {
  if (!days.length) return `<p class="muted">Nothing yet.</p>`;
  // every day from the first to today, so gaps show as gaps
  const map = new Map(days.map((d) => [d.day, d]));
  const all = [];
  for (let d = new Date(days[0].day + "T00:00:00Z"), end = new Date(); d <= end; d.setUTCDate(d.getUTCDate() + 1)) {
    const k = d.toISOString().slice(0, 10);
    all.push(map.get(k) || { day: k, old_hours: 0, new_hours: 0 });
  }
  const W = 960, H = 220, pad = 28, n = all.length, bw = (W - pad) / n;
  const max = Math.max(1, ...all.map((d) => d.old_hours + d.new_hours));
  const step = niceStep(max);
  const y = (v) => H - 18 - (v / (Math.ceil(max / step) * step)) * (H - 30);
  const grid = [];
  for (let v = 0; v <= Math.ceil(max / step) * step + 1e-9; v += step) {
    grid.push(`<line x1="${pad}" x2="${W}" y1="${y(v)}" y2="${y(v)}" class="grid"/><text x="${pad - 4}" y="${y(v) + 4}" class="ax" text-anchor="end">${v}</text>`);
  }
  const cols = all.map((d, i) => {
    const x = pad + i * bw + 1, w = Math.max(1, bw - 2);
    const o = y(0) - y(d.old_hours), nw = y(0) - y(d.new_hours);
    const tip = `${d.day}: ${h1(d.old_hours + d.new_hours)} h (older builds ${h1(d.old_hours)} h, new ${h1(d.new_hours)} h)`;
    const label = i % Math.ceil(n / 10) === 0 ? `<text x="${x + w / 2}" y="${H - 4}" class="ax" text-anchor="middle">${d.day.slice(5)}</text>` : "";
    return `<g class="col"><title>${tip}</title><rect x="${pad + i * bw}" y="0" width="${bw}" height="${H - 18}" class="hit"/>
      ${d.old_hours ? `<rect x="${x}" y="${y(d.old_hours)}" width="${w}" height="${Math.max(0, o - (d.new_hours ? 1 : 0))}" class="s1" rx="1"/>` : ""}
      ${d.new_hours ? `<rect x="${x}" y="${y(d.old_hours) - nw}" width="${w}" height="${Math.max(0, nw - 1)}" class="s2" rx="1"/>` : ""}${label}</g>`;
  }).join("");
  return `<svg viewBox="0 0 ${W} ${H}" class="chart" role="img" aria-label="Hours played per day">${grid.join("")}${cols}</svg>
    <div class="legend"><span><i class="s1"></i>Older builds (GoatCounter)</span><span><i class="s2"></i>New builds (this counter)</span></div>`;
}

function niceStep(max) {
  const raw = max / 4, p = 10 ** Math.floor(Math.log10(raw));
  return [1, 2, 5, 10].map((m) => m * p).find((s) => s >= raw) || p * 10;
}

export function page(s) {
  const t = s.totals, ob = s.old_builds;
  const lastWeeks = ob.weeks.slice(-8).map((w) => ({ label: `w/c ${w.week.slice(5)}`, value: w.hours }));
  const tiles = [
    ["Hours played, all time", hrs(t.hours)],
    ["Last 7 days", hrs(t.hours_7d)],
    ["Last 30 days", hrs(t.hours_30d)],
    ["Launches counted here", t.runs.toLocaleString("en-GB")],
    ["Typical session", t.median_session_marks == null ? "–" : t.median_session_marks === 0 ? "under 15 min" : `${t.median_session_marks * 15}–${t.median_session_marks * 15 + 15} min`],
  ];
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>OpenSGolf play time</title><meta name="robots" content="noindex">
<style>
:root{color-scheme:light;--bg:#fcfcfb;--card:#ffffff;--ink:#0b0b0b;--ink2:#52514e;--muted:#8a8984;--line:#e6e5e0;--s1:#2a78d6;--s2:#eb6834}
@media (prefers-color-scheme:dark){:root{color-scheme:dark;--bg:#141413;--card:#1a1a19;--ink:#f0efec;--ink2:#c3c2b7;--muted:#8f8e86;--line:#2c2c2a;--s1:#3987e5;--s2:#d95926}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--ink);font:15px/1.45 system-ui,-apple-system,Segoe UI,sans-serif}
main{max-width:1040px;margin:0 auto;padding:24px 16px 48px}h1{font-size:22px;margin:0 0 4px}h2{font-size:15px;margin:0 0 12px}
.muted,small{color:var(--muted)}.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:12px;margin:20px 0}
.tile,.card{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:14px 16px}.tile b{display:block;font-size:24px;font-weight:600}
.tile span{color:var(--ink2);font-size:13px}.grid2{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:12px;margin-top:12px}
.chart{width:100%;height:auto;display:block}.chart .grid{stroke:var(--line)}.chart .ax{fill:var(--muted);font-size:11px}
.chart .hit{fill:transparent}.chart .col:hover .hit{fill:var(--line);opacity:.5}.s1{fill:var(--s1);background:var(--s1)}.s2{fill:var(--s2);background:var(--s2)}
.legend{display:flex;gap:16px;color:var(--ink2);font-size:13px;margin-top:6px}.legend i{display:inline-block;width:10px;height:10px;border-radius:2px;margin-right:6px}
.bars{display:grid;gap:6px}.bar{display:grid;grid-template-columns:minmax(90px,30%) 1fr auto;gap:10px;align-items:center;font-size:13px}
.bl{color:var(--ink2);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.bt{height:10px}.bf{display:block;height:100%;background:var(--s1);border-radius:0 4px 4px 0;min-width:2px}
.bv{font-variant-numeric:tabular-nums;text-align:right}.bar:hover .bl{color:var(--ink)}
.status{padding:10px 14px;border-radius:8px;border:1px solid var(--line);margin-top:10px}.ok{border-color:#0ca30c}
table{width:100%;border-collapse:collapse;font-size:13px}td,th{text-align:left;padding:4px 6px;border-bottom:1px solid var(--line)}th{color:var(--ink2);font-weight:500}
td.n{text-align:right;font-variant-numeric:tabular-nums}details{margin-top:12px}
</style></head><body><main>
<h1>OpenSGolf play time</h1>
<p class="muted">Updated ${esc(s.generated_at.slice(0, 16).replace("T", " "))} UTC. One count per 15 minutes on the course, so hours are counts ÷ 4.
Store builds (Flathub, F-Droid) send nothing.</p>
<div class="tiles">${tiles.map(([k, v]) => `<div class="tile"><b>${esc(v)}</b><span>${esc(k)}</span></div>`).join("")}</div>

<div class="card"><h2>Hours played per day</h2>${daily(s.daily)}
<details><summary class="muted">As a table</summary><table><tr><th>Day</th><th>Older builds</th><th>New builds</th></tr>
${s.daily.slice().reverse().map((d) => `<tr><td>${d.day}</td><td class="n">${h1(d.old_hours)}</td><td class="n">${h1(d.new_hours)}</td></tr>`).join("")}</table></details></div>

<div class="grid2">
<div class="card"><h2>Hours by platform</h2>${bars(s.platforms.map((p) => ({ label: p.platform, value: p.hours })))}</div>
<div class="card"><h2>Hours by time of day (UTC)</h2>${bars(s.hour_of_day.map((h) => ({ label: `${String(h.hour).padStart(2, "0")}:00`, value: h.hours })))}</div>
<div class="card"><h2>Launches by version</h2>${bars(s.versions.slice(0, 20).map((v) => ({ label: v.version, value: v.launches, note: v.hours ? `${h1(v.hours)} h` : "" })), (n) => n.toLocaleString("en-GB"))}
<p class="muted">Hours by version only for new builds; older builds sent the version only at launch.</p></div>
<div class="card"><h2>Session length (new builds)</h2>${t.runs ? bars(s.session_lengths.map((b) => ({ label: b.label, value: b.runs })), (n) => `${n} runs`) : `<p class="muted">Arrives with the first release that reports here.</p>`}</div>
<div class="card"><h2>Countries (new builds)</h2>${s.countries.length ? bars(s.countries.map((c) => ({ label: c.country, value: c.hours, note: `${c.runs} runs` }))) : `<p class="muted">Arrives with the first release that reports here.</p>`}</div>
<div class="card"><h2>Older builds still reporting to GoatCounter</h2>${lastWeeks.length ? bars(lastWeeks) : `<p class="muted">Nothing imported yet.</p>`}
<div class="status ${ob.can_retire ? "ok" : ""}">${ob.can_retire
    ? "✓ Under 1 hour a week for 3 weeks in a row: the weekly GoatCounter import can be switched off."
    : `This week so far: ${hrs(ob.this_week_hours)}. The import can be switched off once this stays under 1 hour a week for 3 full weeks.`}</div></div>
</div>

<details class="card"><summary><b>Launches in the last 7 days (new builds)</b></summary><table><tr><th>Started (UTC)</th><th>Platform</th><th>Version</th><th>Country</th><th>Played</th></tr>
${s.recent_runs.map((r) => `<tr><td>${esc(r.started_at.slice(0, 16).replace("T", " "))}</td><td>${esc(r.platform)}</td><td>${esc(r.version)}</td><td>${esc(r.country || "–")}</td><td class="n">${r.beats ? `${r.beats * 15}+ min` : "under 15 min"}</td></tr>`).join("")}</table></details>
<details class="card"><summary><b>GoatCounter imports</b></summary><table><tr><th>When (UTC)</th><th>From</th><th>Rows</th><th>Hours in window</th><th>Problem</th></tr>
${s.imports.map((i) => `<tr><td>${esc(i.at.slice(0, 16).replace("T", " "))}</td><td>${esc(i.start_day)}</td><td class="n">${i.rows}</td><td class="n">${h1(i.play_marks / 4)}</td><td>${esc(i.error || "")}</td></tr>`).join("")}</table></details>
</main></body></html>`;
}
