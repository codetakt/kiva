#!/usr/bin/env python3
"""Exercise corrected witnesses against bounded good and single-defect peers.

This peer implements only the selected inputs. It is not a reference LRS or a
proof of the complete Suite. The real Rust runner sends and grades each case.
"""
import argparse
import copy
import datetime as dt
import hashlib
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import subprocess
import threading
from urllib.parse import parse_qs, urlsplit
import uuid

import yaml

from generate_lrs_projection_cases import dump_yaml, STANDARD

ROOT = Path(__file__).resolve().parents[2]
VERSION = 'statement-accepts-semantic-version-outside-legacy-adl-range'
FUTURE = 'statement-future-timestamp-is-not-rejected'
SIGNATURE = 'signature-equivalence-excludes-case'
DUPLICATE = 'issued-completion-identical-duplicate-batch-id'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runner', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    selected = {}
    inputs = {}
    for path in sorted((ROOT / 'tests/conformance/http').rglob('*.yaml')):
        document = yaml.load(path.read_text(), Loader=yaml.CSafeLoader)
        for case in document['cases']:
            name = case['id']
            if name in [VERSION, FUTURE, SIGNATURE, DUPLICATE] or (
                name.startswith('issued-completion-')
                and ('-duration-' in name or '-sub-forbidden-' in name)
            ):
                selected[name] = case
                inputs[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
    assert len(selected) == 46, len(selected)
    suite = args.out / 'cases.yaml'
    suite.write_text(dump_yaml(dict(suite=dict(id='issued-revision-controls',
        standard=STANDARD, description='Bounded correction controls'),
        cases=list(selected.values())), sort_keys=False, allow_unicode=True))
    expected = {
        'baseline': set(),
        'future-margin-10s': set(),
        'reject-version': set(),
        'rewrite-version': {VERSION},
        'accept-identical-duplicates': {DUPLICATE},
        'accept-substatement-fields': {n for n in selected if '-sub-forbidden-' in n},
        'time-only-duration': {n for n in selected if any('-duration-' + k + '-' in n for k in ['calendar', 'week', 'day'])},
        'reject-comma': {n for n in selected if '-duration-comma-' in n},
    }
    results = {}
    for mode, failures in expected.items():
        statements, wire = {}, []

        class Peer(BaseHTTPRequestHandler):
            def log_message(self, *_):
                pass

            def reply(self, status, value=None, stored=None):
                body = b'' if value is None else json.dumps(value).encode()
                self.send_response(status)
                self.send_header('X-Experience-API-Version', '2.0.0')
                self.send_header('X-Experience-API-Consistent-Through', '2020-01-01T00:00:00Z')
                self.send_header('Content-Type', 'application/json')
                self.send_header('Content-Length', str(len(body)))
                if stored:
                    when = dt.datetime.fromisoformat(stored.replace('Z', '+00:00'))
                    self.send_header('Last-Modified', when.strftime('%a, %d %b %Y %H:%M:%S GMT'))
                self.end_headers()
                self.wfile.write(body)

            def do_GET(self):
                query = parse_qs(urlsplit(self.path).query)
                identifier = query.get('statementId', [''])[0]
                wire.append(dict(method='GET', path=self.path))
                if identifier not in statements:
                    self.reply(404)
                    return
                value = copy.deepcopy(statements[identifier])
                if mode == 'rewrite-version':
                    value['version'] = '2.0.0'
                self.reply(200, value, value['stored'])

            def write_statement(self):
                raw = self.rfile.read(int(self.headers.get('Content-Length', '0')))
                wire.append(dict(method=self.command, path=self.path, body_hex=raw.hex()))
                if self.headers.get('Content-Type', '').startswith('multipart/'):
                    # RC-08: diagnostic status 400 may have a nonempty error body.
                    self.reply(400, {'error':'signature comparison is diagnostic'})
                    return
                value = json.loads(raw)
                values = value if isinstance(value, list) else [value]
                ids = [s.get('id') for s in values if 'id' in s]
                if len(ids) != len(set(ids)) and mode != 'accept-identical-duplicates':
                    self.reply(400)
                    return
                for statement in values:
                    child = statement.get('object', {})
                    if child.get('objectType') == 'SubStatement' and mode != 'accept-substatement-fields':
                        if {'id', 'stored', 'authority', 'version'} & child.keys():
                            self.reply(400)
                            return
                    duration = (child if child.get('objectType') == 'SubStatement' else statement).get('result', {}).get('duration')
                    if duration and ((mode == 'time-only-duration' and not duration.startswith('PT'))
                                     or (mode == 'reject-comma' and ',' in duration)):
                        self.reply(400)
                        return
                    if mode == 'reject-version' and statement.get('version') == '1.1.0':
                        self.reply(400)
                        return
                    if mode == 'future-margin-10s' and 'timestamp' in statement:
                        timestamp = dt.datetime.fromisoformat(statement['timestamp'].replace('Z', '+00:00'))
                        if timestamp > dt.datetime.now(dt.timezone.utc) + dt.timedelta(seconds=10):
                            self.reply(400)
                            return
                saved = []
                for statement in values:
                    statement = copy.deepcopy(statement)
                    query = parse_qs(urlsplit(self.path).query)
                    identifier = statement.get('id') or query.get('statementId', [str(uuid.uuid4())])[0]
                    statement['id'] = identifier
                    statement['stored'] = '2020-01-01T00:00:00.000Z'
                    statement.setdefault('timestamp', statement['stored'])
                    statement.setdefault('version', '2.0.0')
                    statement.setdefault('authority', {'objectType':'Agent', 'mbox':'mailto:control@example.org'})
                    statements[identifier] = statement
                    saved.append(identifier)
                self.reply(204 if self.command == 'PUT' else 200, None if self.command == 'PUT' else saved)

            do_PUT = write_statement
            do_POST = write_statement

        server = ThreadingHTTPServer(('127.0.0.1', 0), Peer)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            run = subprocess.run([str(args.runner.resolve()), '--case', str(suite),
                '--endpoint', f'http://127.0.0.1:{server.server_port}',
                '--allow-destructive', '--json'], capture_output=True, text=True)
        finally:
            server.shutdown()
            worker.join()
            server.server_close()
        (args.out / (mode + '.json')).write_text(run.stdout)
        (args.out / (mode + '.stderr')).write_text(run.stderr)
        (args.out / (mode + '.wire.json')).write_text(json.dumps(wire, indent=2) + '\n')
        report = json.loads(run.stdout)
        outcomes = {c['id']:c for s in report['suites'] for c in s['cases']}
        actual = {name for name, c in outcomes.items() if c['outcome'] == 'failed'}
        assert set(outcomes) == set(selected), set(outcomes) ^ set(selected)
        assert actual == failures, (mode, actual ^ failures)
        assert all(c['outcome'] in ['passed', 'failed'] for c in outcomes.values())
        assert run.returncode == (1 if failures else 0), run.returncode
        if mode == 'future-margin-10s':
            assert not any(w['method'] == 'GET' and '000000000305' in w['path'] for w in wire)
        if mode == 'reject-version':
            assert not any(w['method'] == 'GET' and 'retained_version' in w['path'] for w in wire)
        results[mode] = dict(cases=len(outcomes), expected_failed=sorted(failures), actual_failed=sorted(actual),
            messages={n:outcomes[n] for n in actual}, exit_code=run.returncode)
    inputs[str(args.runner)] = hashlib.sha256(args.runner.read_bytes()).hexdigest()
    inputs[str(Path(__file__).relative_to(ROOT))] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    (args.out / 'summary.json').write_text(json.dumps(dict(
        scope='Bounded runtime controls; not an LRS reference implementation or whole-suite proof',
        inputs=inputs, results=results), indent=2) + '\n')
    print(json.dumps({m:dict(cases=r['cases'], failures=len(r['actual_failed'])) for m,r in results.items()}, indent=2))


if __name__ == '__main__':
    main()
