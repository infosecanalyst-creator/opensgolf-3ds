# Reverse engineering

OpenSGolf's rules and pictures come from facts read out of the original program: the publisher's golf.exe (never the
SafeDisc retail exe or a no-CD crack), Terrain.dll and jgl.dll of a user's own install. Decompiled code is read for facts
(numbers, tables, orders, strings, positions) and never copied into this repository; the port's code is written from the
facts and says in a comment where each comes from. The decode notes (docs/DECODE_*.md, UI_*.md, TERRAIN.md and the
PARITY.md rows) record the address and how sure the reading is (EXACT, DERIVED, WEAK, PLACEHOLDER).

## REA with headless Ghidra

[REA](https://github.com/morluto/rea) (npm package `rea-agents`, used here at 6.4.0 with Node 22) drives an analysis
engine from the command line or as an MCP server and returns its results as JSON "Evidence". It works with an existing
Ghidra 12.1.x installation and a 64-bit JDK 21 or newer; it never installs either.

Setup:

```sh
npm install --global rea-agents@6.4.0        # or run any command as: npx -y rea-agents@6.4.0 <command>
export GHIDRA_INSTALL_DIR=/abs/path/ghidra_12.1.4_PUBLIC
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64   # optional when java and javac are on PATH
rea doctor --provider ghidra --json          # "healthy": true once Ghidra, its native decompiler and the JDK check out
```

No `rea setup` is needed for the command line: the provider is configured by `GHIDRA_INSTALL_DIR` alone (setup only
registers REA with agent clients). For the 950 KB golf.exe the first analysis takes about 2.5 minutes, within REA's
default startup deadline of 330 s; raise it with `REA_GHIDRA_STARTUP_TIMEOUT_MS` for larger targets.

Commands used for this project (`--provider ghidra --json`, plus `--snapshot FILE` to keep exact results for reuse):

```sh
rea analyze   /abs/Terrain.dll                         # segments, procedure and string counts
rea search    /abs/Terrain.dll ".bmp"                   # strings: /RetainWallA.bmp at 0x1005f234 ...
rea xrefs     /abs/Terrain.dll 0x1005f234               # -> 0x10007ef8, the texture loader
rea decompile /abs/Terrain.dll 0x1000f834               # the procedure containing an address: the wall draw 0x1000f7f0
rea decompile /abs/Terrain.dll "Terrain::elevateCorner" # exported names work too
rea decompile /abs/jgl.dll 0x1000e920                   # the tint blit
```

Use `--filter-output normalized_result` to get just the pseudocode or the list. Every command line call is a fresh
process that imports and analyses the binary again (about 40 s for Terrain.dll); for a batch of questions on golf.exe,
`tools/rea_session.py` runs them through one `rea --mcp` session, so the analysis is paid once:

```sh
GHIDRA_INSTALL_DIR=... tools/rea_session.py queries.json out/
# queries.json: [{"tool": "open_binary", "args": {"path": "/abs/golf_publisher.exe", "provider_id": "ghidra"}, "name": "open"},
#                {"tool": "procedure_pseudo_code", "args": {"procedure": "0x4659a0"}, "name": "pros"},
#                {"tool": "xrefs", "args": {"address": "0x58dd70"}, "name": "pro_table"},
#                {"tool": "read_bytes", "args": {"address": "0x4c11e0", "length": 64}, "name": "yard_pieces"},
#                {"tool": "close_binary", "args": {}, "name": "close"}]
```

Notes from use:

* REA only decompiles addresses Ghidra made into functions. Code reached only through a vtable (jgl.dll's sprite
  methods, e.g. the paletted blit at 0x100098c0) answers "No procedure exists at the requested address"; read those in a
  plain disassembly (objdump) or let Ghidra's analyzeHeadless with a script create the function first.
* The exe's giant main frame routine (0x40f5c0) decompiles with garbled stack arguments, as it does in Ghidra itself;
  the assembly settles which object or value is passed (e.g. the Customise hover piece at 0x43876e).
* Data that the exe fills at run time (the terrain table at 0x578350) is empty in the file; read its static source
  (0x4c1a40) with `read_bytes` or a PE reader instead.
* Results are JSON Evidence with provenance (target digest, provider version, limitations); `xrefs` returns the
  referencing instruction addresses, `procedure_callers` whole procedures.
