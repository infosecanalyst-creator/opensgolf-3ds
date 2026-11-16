# Play-time stats

A Cloudflare Worker with a D1 database that keeps OpenSGolf's anonymous play-time count (docs/PLAYING.md "Play-time
count", crates/simgolf/src/playcount.rs) and shows it on a password-protected page.

* `GET /count?e=<event>&v=<version>&s=<run id>&n=<mark>`: what the game sends. `e` is `start/<platform>/<version>` once
  at start and `play-15min/<platform>` for each 15 minutes of play; `s` is a random id made at each start and never saved
  (or `kept` for counts sent late); `n` the mark's number in that run. Only the country Cloudflare reads from the address
  is kept, never the address.
* `GET /` and `GET /api/stats`: the stats (basic auth, any user name, the `STATS_PASSWORD` secret).
* Weekly (the cron in `wrangler.toml`) and `POST /admin/import[?from=YYYY-MM-DD]`: GoatCounter's counts from builds
  before the switch, by hour (needs the `GOATCOUNTER_TOKEN` secret). The first import takes everything since 1 Oct 2026;
  later ones re-read the last 15 days, replacing those hours. The page says when older builds have stayed under an hour a
  week for three full weeks: then remove the cron and deploy again.

## Deploy

```sh
cd stats
npx wrangler@4.152.0 d1 create opensgolf-stats        # once; put the id in wrangler.toml
npx wrangler@4.152.0 d1 execute opensgolf-stats --remote --file schema.sql
npx wrangler@4.152.0 secret put GOATCOUNTER_TOKEN
npx wrangler@4.152.0 secret put STATS_PASSWORD
npx wrangler@4.152.0 deploy
```

## Tests

`node --test stats/test/*.test.mjs` (Node 22.5 or later: the tests stand D1 in with node:sqlite).
