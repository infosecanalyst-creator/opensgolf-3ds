#!/usr/bin/env python3
"""Run several REA queries against one binary in a single Ghidra session (REA's MCP server over stdio).

Each `rea <command>` on the command line imports and auto-analyses the binary again (about 40 s for Terrain.dll and
2.5 minutes for the publisher's golf.exe on a laptop-class container); one `rea --mcp` session pays that once.

usage: tools/rea_session.py QUERIES.json OUT_DIR
  QUERIES.json is a list of {"tool": <MCP tool>, "args": {...}, "name": <output file stem>}; start it with an
  open_binary call ({"path": "/abs/path.exe", "provider_id": "ghidra"}) and end it with close_binary. Useful tools:
  procedure_pseudo_code {"procedure": "0x4659a0"}, xrefs {"address": "0x58dd70"}, read_bytes {"address": ..., "length": n},
  procedure_callers / procedure_callees {"procedure": ...}, search_strings {"pattern": ...}, batch_decompile
  {"addresses": [...]}. A "list" tool name writes the server's tool list and schemas instead.
env: GHIDRA_INSTALL_DIR (required), JAVA_HOME (optional), REA (the rea command, default "rea").
Every result is written to OUT_DIR/<name>.txt as the JSON text REA returns.
"""
import json
import os
import subprocess
import sys
import time


def main():
    qfile, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    env = dict(os.environ)
    env.setdefault('REA_GHIDRA_STARTUP_TIMEOUT_MS', '3600000')
    env.setdefault('REA_ANALYSIS_PROVIDER', 'ghidra')
    proc = subprocess.Popen([env.get('REA', 'rea'), '--mcp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                            stderr=open(os.path.join(out, 'stderr.log'), 'w'), env=env, text=True, bufsize=1)
    next_id = [0]

    def call(method, params=None, notify=False):
        msg = {'jsonrpc': '2.0', 'method': method}
        if params is not None:
            msg['params'] = params
        if not notify:
            next_id[0] += 1
            msg['id'] = next_id[0]
        proc.stdin.write(json.dumps(msg) + '\n')
        proc.stdin.flush()
        if notify:
            return None
        while True:
            line = proc.stdout.readline()
            if not line:
                raise SystemExit('rea --mcp closed the session')
            try:
                reply = json.loads(line)
            except ValueError:
                continue
            if reply.get('id') == next_id[0]:
                return reply

    call('initialize', {'protocolVersion': '2025-06-18', 'capabilities': {}, 'clientInfo': {'name': 'rea_session', 'version': '1'}})
    call('notifications/initialized', notify=True)
    for q in json.load(open(qfile)):
        start = time.time()
        if q['tool'] == 'list':
            tools = call('tools/list', {})['result']['tools']
            with open(os.path.join(out, 'tools.txt'), 'w') as f:
                f.write('\n'.join(t['name'] + ': ' + t.get('description', '') for t in tools))
            with open(os.path.join(out, 'schemas.json'), 'w') as f:
                json.dump({t['name']: t.get('inputSchema') for t in tools}, f, indent=1)
            print('tools', len(tools))
            continue
        reply = call('tools/call', {'name': q['tool'], 'arguments': q['args']})
        res = reply.get('result', reply.get('error'))
        if isinstance(res, dict) and 'content' in res:
            text = '\n'.join(c.get('text', '') for c in res['content'] if c.get('type') == 'text')
        else:
            text = json.dumps(res)
        with open(os.path.join(out, q['name'] + '.txt'), 'w') as f:
            f.write(text)
        print('%s: %.0f s, %d bytes' % (q['name'], time.time() - start, len(text)), flush=True)
    proc.stdin.close()
    proc.wait(timeout=600)


if __name__ == '__main__':
    main()
