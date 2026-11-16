// OpenSGolf's play-time stats (stats/README.md): a Cloudflare Worker with a D1 database.
//
// * GET /count?e=<event>&v=<version>&s=<run id>&n=<mark>: the game's play-time count (crates/simgolf/src/playcount.rs).
//   `e` is start/<platform>/<version> once at start or play-15min/<platform> at each 15 minutes of play; `s` a random id the
//   game makes at each start and never saves (or "kept" for marks sent late, with no run); `n` the mark's number in the run.
//   Nothing else is kept: no address, only the country Cloudflare reads from it.
// * Every week (the cron in wrangler.toml), and on POST /admin/import: GoatCounter's counts from older builds, by hour.
// * GET / (password): the stats page; GET /api/stats the same as JSON.
//
// Copyright the OpenSGolf contributors. GPL-2.0-or-later.

/** Platforms the game names (playcount.rs `platform`). */
export const PLATFORMS = new Set([
  "windows", "macos", "linux", "android", "web", "other",
  "windows-arm64", "macos-arm64", "linux-arm64", "other-arm64",
]);
/** GoatCounter's counter (the old builds' COUNT_URL). */
export const GOATCOUNTER = "https://opensgolf.goatcounter.com";
/** The first day an import with nothing yet imported starts from (before the first play-time count). */
export const FIRST_DAY = "2026-10-01";
/** How far back the weekly import looks again (late counts from older builds land in recent hours). */
export const REIMPORT_DAYS = 15;
/** Old builds' weekly play below this many hours, for [RETIRE_WEEKS] full weeks in a row: the import can be retired. */
export const RETIRE_HOURS = 1;
export const RETIRE_WEEKS = 3;

/** An event's parts, or null when it is not one of the game's. */
export function parseEvent(e) {
  if (typeof e !== "string" || e.length > 64) return null;
  const p = e.split("/");
  if (p[0] === "start" && p.length === 3 && PLATFORMS.has(p[1]) && isVersion(p[2])) {
    return { kind: "start", platform: p[1], version: p[2] };
  }
  if (p[0] === "play-15min" && p.length === 2 && PLATFORMS.has(p[1])) return { kind: "play", platform: p[1] };
  return null;
}

export function isVersion(v) {
  return typeof v === "string" && /^\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(v);
}

/** A run id: 8 to 32 hex digits. */
export function isRunId(s) {
  return typeof s === "string" && /^[0-9a-f]{8,32}$/.test(s);
}

/** 'YYYY-MM-DDTHH' of a Date, in UTC. */
export function hourOf(d) {
  return d.toISOString().slice(0, 13);
}

/** A count's request, checked: what to store, or null to ignore it. */
export function readCount(url, country) {
  const q = url.searchParams;
  const ev = parseEvent(q.get("e"));
  if (!ev) return null;
  const version = ev.kind === "start" ? ev.version : q.get("v");
  if (!isVersion(version)) return null;
  const s = q.get("s") || "";
  const kept = s === "kept";
  if (!kept && !isRunId(s)) return null;
  const n = Number.parseInt(q.get("n") || "0", 10);
  if (!Number.isFinite(n) || n < 0 || n > 100000) return null;
  const c = typeof country === "string" && /^[A-Z0-9]{2}$/.test(country) ? country : "";
  return { kind: ev.kind, platform: ev.platform, version, sid: kept ? null : s, n, country: c };
}

/** Stores one count. A mark already counted for its run (a repeat) changes nothing. */
export async function storeCount(db, c, now) {
  const at = now.toISOString();
  const hour = hourOf(now);
  const playRow = (kept) =>
    db.prepare(
      `INSERT INTO play_hourly (hour, platform, version, country, kept, beats) VALUES (?1, ?2, ?3, ?4, ?5, 1)
       ON CONFLICT (hour, platform, version, country, kept) DO UPDATE SET beats = beats + 1`,
    ).bind(hour, c.platform, c.version, c.country, kept);
  if (!c.sid) {
    if (c.kind === "play") await playRow(1).run();
    return;
  }
  await db.prepare(
    `INSERT INTO runs (sid, platform, version, country, started_at, last_at, beats) VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0)
     ON CONFLICT (sid) DO NOTHING`,
  ).bind(c.sid, c.platform, c.version, c.country, at).run();
  if (c.kind !== "play") return;
  const r = await db.prepare(`UPDATE runs SET beats = ?2, last_at = ?3 WHERE sid = ?1 AND beats < ?2`)
    .bind(c.sid, Math.max(1, c.n), at).run();
  if (r.meta && r.meta.changes > 0) await playRow(0).run();
}

// ---------------------------------------------------------------- GoatCounter import

/** GoatCounter paths worth keeping: the game's own events (not the page's visits or start-button clicks). */
export function gameGcPath(path) {
  return typeof path === "string" && (path.startsWith("play-15min/") || path.startsWith("start/"));
}

/** Rows [hour, path, count] from one page of GoatCounter's /api/v0/stats/hits, only hours on or after `fromDay`. */
export function gcRows(hits, fromDay) {
  const rows = [];
  for (const h of hits || []) {
    if (!gameGcPath(h.path)) continue;
    for (const s of h.stats || []) {
      if (!s.day || s.day < fromDay || !Array.isArray(s.hourly)) continue;
      s.hourly.forEach((n, i) => {
        if (n > 0) rows.push([`${s.day}T${String(i).padStart(2, "0")}`, h.path, n]);
      });
    }
  }
  return rows;
}

/** Imports GoatCounter's counts from `fromDay` to now; re-imported hours replace what was there. */
export async function importGoatCounter(env, fromDay, now = new Date(), fetcher = fetch) {
  const db = env.DB;
  let rows = [];
  try {
    if (!env.GOATCOUNTER_TOKEN) throw new Error("GOATCOUNTER_TOKEN is not set");
    const seen = [];
    for (let page = 0; page < 20; page++) {
      const u = new URL(`${GOATCOUNTER}/api/v0/stats/hits`);
      u.searchParams.set("start", `${fromDay}T00:00:00Z`);
      u.searchParams.set("end", now.toISOString().slice(0, 19) + "Z");
      u.searchParams.set("limit", "100");
      if (seen.length) u.searchParams.set("exclude_paths", seen.join(","));
      const res = await fetcher(u.toString(), { headers: { Authorization: `Bearer ${env.GOATCOUNTER_TOKEN}` } });
      if (!res.ok) throw new Error(`GoatCounter answered ${res.status}`);
      const d = await res.json();
      rows = rows.concat(gcRows(d.hits, fromDay));
      for (const h of d.hits || []) seen.push(h.path_id);
      if (!d.more || !(d.hits || []).length) break;
    }
    // one statement per 1,000 rows (D1 limits statements per run and parameters per statement, not a JSON value's size)
    const stmts = [];
    for (let i = 0; i < rows.length; i += 1000) {
      stmts.push(db.prepare(
        `INSERT INTO gc_hourly (hour, path, count)
         SELECT j.value ->> 0, j.value ->> 1, j.value ->> 2 FROM json_each(?1) AS j WHERE true
         ON CONFLICT (hour, path) DO UPDATE SET count = excluded.count`,
      ).bind(JSON.stringify(rows.slice(i, i + 1000))));
    }
    if (stmts.length) await db.batch(stmts);
    const marks = rows.filter((r) => r[1].startsWith("play-15min/")).reduce((a, r) => a + r[2], 0);
    await logImport(db, now, fromDay, rows.length, marks, null);
    return { from: fromDay, rows: rows.length, play_marks: marks };
  } catch (e) {
    await logImport(db, now, fromDay, rows.length, 0, String(e && e.message || e));
    return { from: fromDay, error: String(e && e.message || e) };
  }
}

async function logImport(db, now, fromDay, rows, marks, error) {
  await db.prepare(`INSERT INTO gc_imports (at, start_day, end_day, rows, play_marks, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6)`)
    .bind(now.toISOString(), fromDay, now.toISOString().slice(0, 10), rows, marks, error).run();
}

/** Where the weekly import starts: from [FIRST_DAY] when nothing was imported yet, else [REIMPORT_DAYS] back. */
export async function importStart(db, now) {
  const any = await db.prepare(`SELECT 1 AS x FROM gc_hourly LIMIT 1`).first();
  if (!any) return FIRST_DAY;
  return new Date(now.getTime() - REIMPORT_DAYS * 86400e3).toISOString().slice(0, 10);
}

// ---------------------------------------------------------------- stats

/** Session-length buckets, by 15-minute marks reached. */
export const LENGTHS = [
  ["under 15 min", 0, 0], ["15–30 min", 1, 1], ["30 min–1 h", 2, 3], ["1–2 h", 4, 7], ["2–4 h", 8, 15], ["4 h or more", 16, 1e9],
];

/** The stats, from both sources. */
export async function stats(db, now = new Date()) {
  const all = async (sql, ...p) => (await db.prepare(sql).bind(...p).all()).results || [];
  const day30 = new Date(now.getTime() - 30 * 86400e3).toISOString().slice(0, 10);
  const day7 = new Date(now.getTime() - 7 * 86400e3).toISOString().slice(0, 13);

  const daily = await all(`
    SELECT day, SUM(old) AS old, SUM(new) AS new FROM (
      SELECT substr(hour, 1, 10) AS day, SUM(count) AS old, 0 AS new FROM gc_hourly WHERE path LIKE 'play-15min/%' GROUP BY day
      UNION ALL
      SELECT substr(hour, 1, 10) AS day, 0, SUM(beats) FROM play_hourly GROUP BY day
    ) GROUP BY day ORDER BY day`);
  const platforms = await all(`
    SELECT platform, SUM(marks) AS marks FROM (
      SELECT substr(path, 12) AS platform, SUM(count) AS marks FROM gc_hourly WHERE path LIKE 'play-15min/%' GROUP BY platform
      UNION ALL SELECT platform, SUM(beats) FROM play_hourly GROUP BY platform
    ) GROUP BY platform ORDER BY marks DESC`);
  // launches by version: GoatCounter's start/<platform>/<version> and this Worker's runs
  const launches = await all(`
    SELECT version, SUM(n) AS launches FROM (
      SELECT substr(path, instr(substr(path, 7), '/') + 7) AS version, SUM(count) AS n FROM gc_hourly WHERE path LIKE 'start/%' GROUP BY version
      UNION ALL SELECT version, COUNT(*) FROM runs GROUP BY version
    ) GROUP BY version`);
  const versionHours = await all(`SELECT version, SUM(beats) AS marks FROM play_hourly GROUP BY version`);
  const hourOfDay = await all(`
    SELECT h, SUM(marks) AS marks FROM (
      SELECT CAST(substr(hour, 12, 2) AS INTEGER) AS h, SUM(count) AS marks FROM gc_hourly WHERE path LIKE 'play-15min/%' GROUP BY h
      UNION ALL SELECT CAST(substr(hour, 12, 2) AS INTEGER), SUM(beats) FROM play_hourly GROUP BY 1
    ) GROUP BY h ORDER BY h`);
  const lengths = await all(`SELECT beats, COUNT(*) AS runs FROM runs GROUP BY beats ORDER BY beats`);
  const countries = await all(`
    SELECT country, COUNT(*) AS runs, SUM(beats) AS marks FROM runs GROUP BY country ORDER BY marks DESC, runs DESC LIMIT 25`);
  const recentRuns = await all(`
    SELECT platform, version, country, started_at, beats FROM runs WHERE started_at >= ?1 ORDER BY started_at DESC LIMIT 50`, day7);
  const oldWeekly = await all(`
    SELECT date(substr(hour, 1, 10), '-6 days', 'weekday 1') AS week, SUM(count) AS marks
    FROM gc_hourly WHERE path LIKE 'play-15min/%' GROUP BY week ORDER BY week`);
  const imports = await all(`SELECT at, start_day, rows, play_marks, error FROM gc_imports ORDER BY at DESC LIMIT 8`);
  const runs7 = await db.prepare(`SELECT COUNT(*) AS n FROM runs WHERE started_at >= ?1`).bind(day7).first();

  const lengthBuckets = LENGTHS.map(([label, lo, hi]) => ({
    label, runs: lengths.filter((r) => r.beats >= lo && r.beats <= hi).reduce((a, r) => a + r.runs, 0),
  }));
  const totalRuns = lengths.reduce((a, r) => a + r.runs, 0);
  const sorted = lengths.flatMap((r) => Array(Math.min(r.runs, 100000)).fill(r.beats));
  const median = sorted.length ? sorted[Math.floor(sorted.length / 2)] : null;

  const vh = new Map(versionHours.map((r) => [r.version, r.marks]));
  const versions = launches.map((r) => ({ version: r.version, launches: r.launches, hours: (vh.get(r.version) || 0) / 4 }))
    .sort((a, b) => cmpVersion(b.version, a.version));

  const totalMarks = daily.reduce((a, d) => a + d.old + d.new, 0);
  const last7Marks = daily.filter((d) => d.day >= day7.slice(0, 10)).reduce((a, d) => a + d.old + d.new, 0);
  const last30Marks = daily.filter((d) => d.day >= day30).reduce((a, d) => a + d.old + d.new, 0);

  return {
    generated_at: now.toISOString(),
    totals: {
      hours: totalMarks / 4, hours_7d: last7Marks / 4, hours_30d: last30Marks / 4,
      old_hours: daily.reduce((a, d) => a + d.old, 0) / 4, new_hours: daily.reduce((a, d) => a + d.new, 0) / 4,
      runs: totalRuns, runs_7d: runs7 ? runs7.n : 0, median_session_marks: median,
    },
    daily: daily.map((d) => ({ day: d.day, old_hours: d.old / 4, new_hours: d.new / 4 })),
    platforms: platforms.map((p) => ({ platform: p.platform, hours: p.marks / 4 })),
    versions,
    hour_of_day: Array.from({ length: 24 }, (_, h) => ({ hour: h, hours: (hourOfDay.find((r) => r.h === h) || { marks: 0 }).marks / 4 })),
    session_lengths: lengthBuckets,
    countries: countries.map((c) => ({ country: c.country || "unknown", runs: c.runs, hours: c.marks / 4 })),
    recent_runs: recentRuns,
    old_builds: retirement(oldWeekly, now),
    imports,
  };
}

/** Old builds' play per full week (Monday to Sunday), and whether the import can be retired. */
export function retirement(oldWeekly, now) {
  const thisWeek = new Date(now.getTime());
  thisWeek.setUTCDate(thisWeek.getUTCDate() - ((thisWeek.getUTCDay() + 6) % 7));
  const current = thisWeek.toISOString().slice(0, 10);
  const weeks = oldWeekly.map((w) => ({ week: w.week, hours: w.marks / 4 }));
  // full weeks only: weeks with no counts at all are the best sign, so fill the gaps up to last week
  const full = [];
  if (weeks.length) {
    for (let d = new Date(weeks[0].week + "T00:00:00Z"); d.toISOString().slice(0, 10) < current; d.setUTCDate(d.getUTCDate() + 7)) {
      const k = d.toISOString().slice(0, 10);
      full.push({ week: k, hours: (weeks.find((w) => w.week === k) || { hours: 0 }).hours });
    }
  }
  const tail = full.slice(-RETIRE_WEEKS);
  const canRetire = tail.length === RETIRE_WEEKS && tail.every((w) => w.hours < RETIRE_HOURS);
  return { weeks: full, this_week_hours: (weeks.find((w) => w.week === current) || { hours: 0 }).hours, can_retire: canRetire };
}

export function cmpVersion(a, b) {
  const pa = String(a).split(".").map(Number), pb = String(b).split(".").map(Number);
  for (let i = 0; i < 3; i++) if ((pa[i] || 0) !== (pb[i] || 0)) return (pa[i] || 0) - (pb[i] || 0);
  return 0;
}

// ---------------------------------------------------------------- the Worker

/** Basic auth against STATS_PASSWORD (any user name). */
export function authorized(request, env) {
  if (!env.STATS_PASSWORD) return false;
  const h = request.headers.get("Authorization") || "";
  if (!h.startsWith("Basic ")) return false;
  let pass = "";
  try { pass = atob(h.slice(6)).split(":").slice(1).join(":"); } catch { return false; }
  const a = new TextEncoder().encode(pass), b = new TextEncoder().encode(env.STATS_PASSWORD);
  let diff = a.length ^ b.length;
  for (let i = 0; i < Math.max(a.length, b.length); i++) diff |= (a[i] || 0) ^ (b[i] || 0);
  return diff === 0;
}

const CORS = { "Access-Control-Allow-Origin": "*", "Cache-Control": "no-store" };
const ASK = () => new Response("Password needed", { status: 401, headers: { "WWW-Authenticate": 'Basic realm="OpenSGolf stats"' } });

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    if (url.pathname === "/count") {
      if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: { ...CORS, "Access-Control-Allow-Methods": "GET, POST" } });
      const c = readCount(url, request.cf && request.cf.country);
      if (c) await storeCount(env.DB, c, new Date());
      // the same answer whether it was kept or not: the game only looks at whether it got through
      return new Response(null, { status: 204, headers: CORS });
    }
    if (url.pathname === "/robots.txt") return new Response("User-agent: *\nDisallow: /\n");
    if (!authorized(request, env)) return ASK();
    if (url.pathname === "/api/stats") {
      return Response.json(await stats(env.DB), { headers: { "Cache-Control": "no-store" } });
    }
    if (url.pathname === "/admin/import" && request.method === "POST") {
      const from = url.searchParams.get("from");
      const day = from && /^\d{4}-\d{2}-\d{2}$/.test(from) ? from : await importStart(env.DB, new Date());
      return Response.json(await importGoatCounter(env, day));
    }
    if (url.pathname === "/") {
      const { page } = await import("./page.js");
      return new Response(page(await stats(env.DB)), { headers: { "Content-Type": "text/html; charset=utf-8", "Cache-Control": "no-store" } });
    }
    return new Response("Not found", { status: 404 });
  },

  async scheduled(event, env, ctx) {
    const now = new Date();
    ctx.waitUntil(importStart(env.DB, now).then((day) => importGoatCounter(env, day, now)));
  },
};
