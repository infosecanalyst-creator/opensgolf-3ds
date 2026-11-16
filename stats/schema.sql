-- The play-time stats database (stats/README.md). Safe to run again: everything is IF NOT EXISTS.

-- One row per launch of a game that reports here: `sid` is a random number the game makes at start and never saves, so a
-- launch can be told from another but not linked to a player. `beats` is the number of 15-minute marks reached.
CREATE TABLE IF NOT EXISTS runs (
  sid        TEXT PRIMARY KEY,
  platform   TEXT NOT NULL,
  version    TEXT NOT NULL,
  country    TEXT NOT NULL DEFAULT '',
  started_at TEXT NOT NULL,          -- ISO time (UTC) of the first count of the run
  last_at    TEXT NOT NULL,          -- ISO time (UTC) of the latest count
  beats      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS runs_started ON runs (started_at);

-- 15-minute marks per hour (UTC), from every source that reports here. `kept` counts are marks that could not be sent at
-- the time (no internet) and came later with another run: they belong to no run.
CREATE TABLE IF NOT EXISTS play_hourly (
  hour     TEXT NOT NULL,            -- 'YYYY-MM-DDTHH' (UTC)
  platform TEXT NOT NULL,
  version  TEXT NOT NULL,
  country  TEXT NOT NULL DEFAULT '',
  kept     INTEGER NOT NULL DEFAULT 0,
  beats    INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (hour, platform, version, country, kept)
);

-- GoatCounter's counts per hour and event path (play-15min/<platform>, start/<platform>/<version>), as older builds
-- reported them. Re-imported hours replace what was there, so an import can be run again safely.
CREATE TABLE IF NOT EXISTS gc_hourly (
  hour  TEXT NOT NULL,               -- 'YYYY-MM-DDTHH' as GoatCounter gives it (the site's time zone, Europe/London)
  path  TEXT NOT NULL,
  count INTEGER NOT NULL,
  PRIMARY KEY (hour, path)
);

-- Each GoatCounter import.
CREATE TABLE IF NOT EXISTS gc_imports (
  at         TEXT NOT NULL,          -- when it ran (UTC)
  start_day  TEXT NOT NULL,
  end_day    TEXT NOT NULL,
  rows       INTEGER NOT NULL,
  play_marks INTEGER NOT NULL,       -- play-15min counts in the window
  error      TEXT
);
