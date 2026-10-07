#!/usr/bin/env python3
"""Mutating reverse proxy: emulates one LRS defect per run in front of a real LRS.
usage: proxy.py LISTEN_PORT UPSTREAM_PORT MUTATION
"""
import sys, json, re, http.client, socketserver
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlsplit, parse_qsl, urlencode

LISTEN, UP, MUT = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
KNOWN = {
    '/statements': {'statementId', 'voidedStatementId', 'agent', 'verb', 'activity', 'registration', 'related_activities',
                    'related_agents', 'since', 'until', 'limit', 'format', 'attachments', 'ascending'},
    '/activities/state': {'activityId', 'agent', 'registration', 'stateId', 'since'},
    '/agents': {'agent'}, '/activities': {'activityId'},
    '/agents/profile': {'agent', 'profileId', 'since'},
    '/activities/profile': {'activityId', 'profileId', 'since'},
    '/about': set(),
}
DOCS = ('/activities/state', '/agents/profile', '/activities/profile')
CONST_ETAG = '"constant-etag"'
UUID = re.compile(r'[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}')
def upstream(method, path, query, headers, body):
    c = http.client.HTTPConnection('127.0.0.1', UP, timeout=60)
    try:
        c.request(method, path + ('?' + urlencode(query) if query else ''), body=body if (body or method in ('PUT', 'POST')) else None, headers=headers)
        r = c.getresponse(); b = r.read()
        return r.status, r.getheaders(), b
    finally:
        c.close()
HOP = {'connection', 'keep-alive', 'proxy-authenticate', 'proxy-authorization', 'te', 'trailers', 'transfer-encoding', 'upgrade', 'content-length'}

def mutate_request(method, path, query, headers, body):
    """query: list of (k, v). Returns possibly modified tuple."""
    m = MUT
    if m == 'none':
        pass
    elif m == 'ignore-unknown-params':
        if path in KNOWN:
            query = [(k, v) for k, v in query if k in KNOWN[path]]
    elif m.startswith('ignore-unknown-params@'):
        # ignore-unknown-params@METHOD:/path
        mm, pp = m.split('@', 1)[1].split(':', 1)
        if path == pp and method == mm:
            query = [(k, v) for k, v in query if k in KNOWN[path]]
    elif m.startswith('version-as@'):
        # version-as@X=Y : treat request version header X as if it were Y
        x, y = m.split('@', 1)[1].split('=', 1)
        for k in list(headers):
            if k.lower() == 'x-experience-api-version' and headers[k] == x:
                headers[k] = y
    elif m == 'ignore-format-on-lists':
        if path == '/statements' and method in ('GET', 'HEAD') and not any(k in ('statementId', 'voidedStatementId') for k, _ in query):
            query = [(k, v) for k, v in query if k != 'format']
    elif m == 'ignore-malformed-registration-state':
        if path == '/activities/state':
            query = [(k, v) for k, v in query if not (k == 'registration' and not UUID.fullmatch(v))]
    elif m == 'post-ignore-content-type':
        if method == 'POST' and path in DOCS:
            for k in list(headers):
                if k.lower() == 'content-type': headers[k] = 'application/json'
    elif m == 'const-etag':
        if path in DOCS:
            for k in list(headers):
                if k.lower() in ('if-match', 'if-none-match') and CONST_ETAG in headers[k]:
                    st, hs, _ = upstream('GET', path, query, {h: v for h, v in headers.items() if h.lower() not in ('if-match', 'if-none-match', 'content-type')}, b'')
                    real = dict((a.lower(), b) for a, b in hs).get('etag')
                    if st == 200 and real: headers[k] = headers[k].replace(CONST_ETAG, real)
    elif m == 'if-match-ignored-when-absent':
        if method in ('POST', 'DELETE') and path in DOCS and any(k.lower() == 'if-match' for k in headers):
            st, hs, _ = upstream('GET', path, query, {h: v for h, v in headers.items() if h.lower() not in ('if-match', 'if-none-match', 'content-type')}, b'')
            if st == 404: headers = {k: v for k, v in headers.items() if k.lower() != 'if-match'}
    elif m == 'related-agents-drops-agent-filter':
        if path == '/statements' and method in ('GET', 'HEAD') and ('related_agents', 'true') in query:
            query = [(k, v) for k, v in query if k not in ('agent', 'related_agents')]
    return method, path, query, headers, body

def short_circuit(method, path, query, headers, body):
    """Return (status, headers, body) to answer without forwarding, or None."""
    m = MUT
    base = [('X-Experience-API-Version', '2.0.0'), ('Content-Type', 'text/plain')]
    if m == 'post-requires-existing' and method == 'POST' and path in DOCS:
        st, hs, _ = upstream('GET', path, query, {h: v for h, v in headers.items() if h.lower() not in ('if-match', 'if-none-match', 'content-type')}, b'')
        if st == 404: return 404, base, b'no such document'
    if m == 'reject-voided-with-format' and path == '/statements' and method in ('GET', 'HEAD'):
        ks = {k for k, _ in query}
        if 'voidedStatementId' in ks and ks & {'format', 'attachments'}:
            return 400, base + [('X-Experience-API-Consistent-Through', '2026-01-01T00:00:00Z')], b'voidedStatementId cannot be combined'
    if m == 'dup-batch-partial-store' and method == 'POST' and path == '/statements':
        try: arr = json.loads(body)
        except Exception: arr = None
        if isinstance(arr, list) and len(arr) > 1:
            idl = [x.get('id') for x in arr if isinstance(x, dict) and x.get('id')]
            if len(idl) != len(set(idl)):
                upstream('POST', path, query, headers, json.dumps([arr[0]]).encode())
                return 400, base + [('X-Experience-API-Consistent-Through', '2026-01-01T00:00:00Z')], b'duplicate ids in batch'
    return None

def after_forward(method, path, query, headers, status):
    m = MUT
    if m == 'delete-single-deletes-siblings' and method == 'DELETE' and 200 <= status < 300:
        h = {a: b for a, b in headers.items() if a.lower() not in ('if-match', 'if-none-match', 'content-type')}
        if path == '/activities/state' and any(k == 'stateId' for k, _ in query):
            upstream('DELETE', path, [(k, v) for k, v in query if k != 'stateId'], h, b'')
        elif path in ('/agents/profile', '/activities/profile') and any(k == 'profileId' for k, _ in query):
            scope = [(k, v) for k, v in query if k != 'profileId']
            st, hs, b = upstream('GET', path, scope, h, b'')
            if st == 200:
                try: ids = json.loads(b)
                except Exception: ids = []
                for i in ids if isinstance(ids, list) else []:
                    upstream('DELETE', path, scope + [('profileId', i)], dict(h, **{'If-Match': '*'}), b'')

def mutate_response(method, path, query, req_headers, status, headers, body):
    """headers: list of (k, v)."""
    m = MUT
    if m == 'const-etag' and path in DOCS:
        headers = [(k, (CONST_ETAG if k.lower() == 'etag' else v)) for k, v in headers]
    def is_json():
        return any(k.lower() == 'content-type' and 'application/json' in v.lower() for k, v in headers)
    def strip(o, key):
        if isinstance(o, dict): return {k: strip(v, key) for k, v in o.items() if k != key}
        if isinstance(o, list): return [strip(v, key) for v in o]
        return o
    if m == 'doc-last-modified-constant' and path in DOCS:
        headers = [(k, ('Thu, 01 Jan 2015 00:00:00 GMT' if k.lower() == 'last-modified' else v)) for k, v in headers]
    if m == 'conflict-body-generic' and status == 409:
        headers = [(k, v) for k, v in headers if k.lower() != 'content-type'] + [('Content-Type', 'text/plain')]
        body = b'Conflict'
    if m == 'consistent-through-constant' and path == '/statements':
        headers = [(k, ('1970-01-01T00:00:00Z' if k.lower() == 'x-experience-api-consistent-through' else v)) for k, v in headers]
    if m == 'canonical-strip-definitions' and path == '/statements' and method == 'GET' and ('format', 'canonical') in query and status == 200 and is_json():
        try: body = json.dumps(strip(json.loads(body), 'definition')).encode()
        except Exception: pass
    if m == 'activities-strip-definition' and path == '/activities' and method == 'GET' and status == 200 and is_json():
        try: body = json.dumps(strip(json.loads(body), 'definition')).encode()
        except Exception: pass
    if m == 'person-extra-invalid-member' and path == '/agents' and method == 'GET' and status == 200 and is_json():
        try:
            o = json.loads(body)
            if isinstance(o, dict):
                o['mbox'] = list(o.get('mbox') or []) + ['not-a-mailto-iri']
                body = json.dumps(o).encode()
        except Exception: pass
    if m.startswith('drop-response-header@'):
        # drop-response-header@Header-Name[@METHOD:/path]
        parts = m.split('@')
        name = parts[1].lower()
        scope = parts[2] if len(parts) > 2 else None
        if scope is None or scope == f'{method}:{path}':
            headers = [(k, v) for k, v in headers if k.lower() != name]
    return status, headers, body

class H(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    wbufsize = 65536
    disable_nagle_algorithm = True
    def log_message(self, *a): pass
    def _do(self):
        u = urlsplit(self.path)
        query = parse_qsl(u.query, keep_blank_values=True)
        n = int(self.headers.get('Content-Length') or 0)
        body = self.rfile.read(n) if n else b''
        headers = {k: v for k, v in self.headers.items() if k.lower() not in ('host', 'content-length', 'connection')}
        method, path, query, headers, body = mutate_request(self.command, u.path, query, headers, body)
        sc = short_circuit(method, path, query, headers, body)
        if sc is not None:
            status, raw_headers, rbody = sc
        else:
            status, raw_headers, rbody = upstream(method, path, query, headers, body)
            after_forward(method, path, query, headers, status)
        rheaders = [(k, v) for k, v in raw_headers if k.lower() not in HOP]
        status, rheaders, rbody = mutate_response(method, path, query, headers, status, rheaders, rbody)
        self.send_response_only(status)
        for k, v in rheaders: self.send_header(k, v)
        if method == 'HEAD':
            cl = dict((k.lower(), v) for k, v in raw_headers).get('content-length')
            if cl is not None: self.send_header('Content-Length', cl)
        else:
            self.send_header('Content-Length', str(len(rbody)))
        self.end_headers()
        if method != 'HEAD' and rbody: self.wfile.write(rbody)
        self.wfile.flush()
    do_GET = do_PUT = do_POST = do_DELETE = do_HEAD = do_OPTIONS = _do

class S(ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 256
S(('127.0.0.1', LISTEN), H).serve_forever()
