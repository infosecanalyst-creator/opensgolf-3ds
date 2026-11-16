// A stand-in for D1 over node:sqlite, for the tests (node --test stats/test).
import { DatabaseSync } from "node:sqlite";
import { readFileSync } from "node:fs";

export function memoryDb() {
  const raw = new DatabaseSync(":memory:");
  raw.exec(readFileSync(new URL("../schema.sql", import.meta.url), "utf8"));
  const stmt = (sql, params = []) => ({
    bind: (...p) => stmt(sql, p),
    run: async () => { const r = raw.prepare(sql).run(...params); return { meta: { changes: Number(r.changes) } }; },
    all: async () => ({ results: raw.prepare(sql).all(...params) }),
    first: async () => raw.prepare(sql).get(...params) ?? null,
  });
  return { raw, prepare: (sql) => stmt(sql), batch: async (list) => Promise.all(list.map((s) => s.run())) };
}
