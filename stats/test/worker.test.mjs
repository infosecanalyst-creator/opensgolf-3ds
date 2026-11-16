import test from "node:test";
import assert from "node:assert/strict";
import { memoryDb } from "./d1shim.mjs";
import worker, { readCount, storeCount, gcRows, importGoatCounter, importStart, stats, retirement, FIRST_DAY } from "../src/index.js";
import { page } from "../src/page.js";

const u = (q) => new URL(`https://x.test/count?${q}`);

test("only the game's own counts are read", () => {
  assert.deepEqual(readCount(u("e=start/windows/0.8.2&s=0123abcd&n=0"), "GB"),
    { kind: "start", platform: "windows", version: "0.8.2", sid: "0123abcd", n: 0, country: "GB" });
  assert.equal(readCount(u("e=play-15min/linux&v=0.8.2&s=0123abcd&n=1"), "XX").platform, "linux");
  assert.equal(readCount(u("e=play-15min/web&v=0.8.2&s=kept&n=0"), null).sid, null);
  for (const q of ["e=play-15min/amiga&v=0.8.2&s=0123abcd", "e=play-15min/web&s=0123abcd", "e=start/web/x&s=0123abcd",
    "e=play-15min/web&v=0.8.2&s=NOTHEX!!", "e=play-15min/web&v=0.8.2", "e=other&v=0.8.2&s=0123abcd", "e=play-15min/web&v=0.8.2&s=0123abcd&n=-1"]) {
    assert.equal(readCount(u(q), "GB"), null, q);
  }
  assert.equal(readCount(u("e=start/web/0.8.2&s=0123abcd"), "not a country").country, "");
});

test("runs, marks, repeats and kept counts", async () => {
  const db = memoryDb();
  const t = new Date("2026-11-20T10:05:00Z");
  const c = (q) => readCount(u(q), "GB");
  await storeCount(db, c("e=start/macos-arm64/0.8.2&s=aaaaaaaa"), t);
  await storeCount(db, c("e=play-15min/macos-arm64&v=0.8.2&s=aaaaaaaa&n=1"), t);
  await storeCount(db, c("e=play-15min/macos-arm64&v=0.8.2&s=aaaaaaaa&n=1"), t); // a repeat
  await storeCount(db, c("e=play-15min/macos-arm64&v=0.8.2&s=aaaaaaaa&n=2"), new Date("2026-11-20T11:01:00Z"));
  await storeCount(db, c("e=play-15min/web&v=0.8.2&s=bbbbbbbb&n=1"), t); // no start seen: the run is made
  await storeCount(db, c("e=play-15min/windows&v=0.8.1&s=kept"), t);
  const runs = db.raw.prepare("SELECT sid, beats FROM runs ORDER BY sid").all();
  assert.deepEqual(runs.map((r) => [r.sid, r.beats]), [["aaaaaaaa", 2], ["bbbbbbbb", 1]]);
  const marks = db.raw.prepare("SELECT SUM(beats) AS n, SUM(kept * beats) AS k FROM play_hourly").get();
  assert.equal(marks.n, 4);
  assert.equal(marks.k, 1);
});

const gcPage = {
  more: false,
  hits: [
    { path_id: 1, path: "play-15min/windows", stats: [{ day: "2026-11-14", hourly: [0, 3, ...Array(22).fill(0)] }, { day: "2026-09-30", hourly: [5, ...Array(23).fill(0)] }] },
    { path_id: 2, path: "start/windows/0.8.1", stats: [{ day: "2026-11-14", hourly: [1, 2, ...Array(22).fill(0)] }] },
    { path_id: 3, path: "/opensgolf", stats: [{ day: "2026-11-14", hourly: [9, ...Array(23).fill(0)] }] },
    { path_id: 4, path: "start-free-graphics", stats: [{ day: "2026-11-14", hourly: [9, ...Array(23).fill(0)] }] },
  ],
};

test("GoatCounter rows: the game's events only, from the start day", () => {
  assert.deepEqual(gcRows(gcPage.hits, "2026-10-01"), [
    ["2026-11-14T01", "play-15min/windows", 3], ["2026-11-14T00", "start/windows/0.8.1", 1], ["2026-11-14T01", "start/windows/0.8.1", 2],
  ]);
});

test("the import replaces hours, pages, and logs problems", async () => {
  const db = memoryDb();
  const env = { DB: db, GOATCOUNTER_TOKEN: "t" };
  const now = new Date("2026-11-16T04:00:00Z");
  assert.equal(await importStart(db, now), FIRST_DAY);
  const urls = [];
  const fetcher = async (url, init) => {
    urls.push(url);
    assert.equal(init.headers.Authorization, "Bearer t");
    const first = !url.includes("exclude_paths");
    return new Response(JSON.stringify(first ? { ...gcPage, more: true } : { more: false, hits: [] }));
  };
  const r = await importGoatCounter(env, FIRST_DAY, now, fetcher);
  assert.equal(r.rows, 3);
  assert.equal(r.play_marks, 3);
  assert.ok(urls[1].includes("exclude_paths=1%2C2%2C3%2C4"), urls[1]);
  // again: same counts, not doubled
  await importGoatCounter(env, FIRST_DAY, now, fetcher);
  assert.equal(db.raw.prepare("SELECT SUM(count) AS n FROM gc_hourly").get().n, 6);
  assert.equal(await importStart(db, now), "2026-11-01");
  const bad = await importGoatCounter({ DB: db }, FIRST_DAY, now, fetcher);
  assert.match(bad.error, /GOATCOUNTER_TOKEN/);
  assert.equal(db.raw.prepare("SELECT COUNT(*) AS n FROM gc_imports WHERE error IS NOT NULL").get().n, 1);
});

test("stats from both sources, and the page draws", async () => {
  const db = memoryDb();
  await importGoatCounter({ DB: db, GOATCOUNTER_TOKEN: "t" }, FIRST_DAY, new Date("2026-11-16T04:00:00Z"), async () => new Response(JSON.stringify(gcPage)));
  const c = (q) => readCount(u(q), "DE");
  await storeCount(db, c("e=start/linux/0.8.2&s=cccccccc"), new Date("2026-11-16T09:00:00Z"));
  for (let n = 1; n <= 5; n++) await storeCount(db, c(`e=play-15min/linux&v=0.8.2&s=cccccccc&n=${n}`), new Date("2026-11-16T10:00:00Z"));
  await storeCount(db, c("e=start/web/0.8.2&s=dddddddd"), new Date("2026-11-16T09:00:00Z"));
  const s = await stats(db, new Date("2026-11-16T12:00:00Z"));
  assert.equal(s.totals.hours, 2); // 3 old marks + 5 new
  assert.equal(s.totals.old_hours, 0.75);
  assert.equal(s.totals.runs, 2);
  assert.deepEqual(s.platforms.map((p) => [p.platform, p.hours]), [["linux", 1.25], ["windows", 0.75]]);
  assert.deepEqual(s.versions.map((v) => [v.version, v.launches, v.hours]), [["0.8.2", 2, 1.25], ["0.8.1", 3, 0]]);
  assert.deepEqual(s.session_lengths.filter((b) => b.runs).map((b) => [b.label, b.runs]), [["under 15 min", 1], ["1–2 h", 1]]);
  assert.equal(s.countries[0].country, "DE");
  assert.equal(s.hour_of_day[1].hours, 0.75);
  const html = page(s);
  assert.match(html, /OpenSGolf play time/);
  assert.match(html, /2 h/);
});

test("retiring the import: three full quiet weeks", () => {
  const now = new Date("2026-12-09T12:00:00Z"); // a Wednesday; this week starts 7 Dec
  const busy = [{ week: "2026-11-02", marks: 1400 }, { week: "2026-11-09", marks: 200 }];
  assert.equal(retirement(busy, now).can_retire, true, "weeks with nothing count as quiet");
  assert.equal(retirement([...busy, { week: "2026-11-30", marks: 8 }], now).can_retire, false, "2 h last week");
  assert.equal(retirement([...busy, { week: "2026-11-30", marks: 3 }, { week: "2026-12-07", marks: 50 }], now).can_retire, true);
  assert.equal(retirement([], now).can_retire, false);
});

test("the Worker: counts are open, the rest needs the password", async () => {
  const db = memoryDb();
  const env = { DB: db, STATS_PASSWORD: "pw" };
  const r = await worker.fetch(new Request("https://x.test/count?e=start/web/0.8.2&s=eeeeeeee"), env, {});
  assert.equal(r.status, 204);
  assert.equal(r.headers.get("Access-Control-Allow-Origin"), "*");
  assert.equal((await worker.fetch(new Request("https://x.test/count?e=junk"), env, {})).status, 204);
  assert.equal((await worker.fetch(new Request("https://x.test/"), env, {})).status, 401);
  const wrong = { Authorization: "Basic " + btoa("x:nope") }, right = { Authorization: "Basic " + btoa("x:pw") };
  assert.equal((await worker.fetch(new Request("https://x.test/", { headers: wrong }), env, {})).status, 401);
  const ok = await worker.fetch(new Request("https://x.test/api/stats", { headers: right }), env, {});
  assert.equal((await ok.json()).totals.runs, 1);
  assert.equal((await worker.fetch(new Request("https://x.test/", { headers: right }), { DB: db }, {})).status, 401, "no password set: closed");
});
