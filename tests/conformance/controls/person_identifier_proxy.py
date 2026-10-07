"""One response defect: add an enriched Person openid with malformed URI syntax.

The valid contrast uses the same additional array index with a legal IP literal.
No Suite cases, expected values or Kiva implementation are read by this proxy.
"""
import argparse
import http.client
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

parser = argparse.ArgumentParser()
parser.add_argument('--listen', type=int, required=True)
parser.add_argument('--upstream', type=int, required=True)
parser.add_argument('--mode', choices=['valid', 'invalid'], required=True)
parser.add_argument('--trace', type=Path, required=True)
args = parser.parse_args()
added = 'https://[2001:db8::1]/person' if args.mode == 'valid' else 'https://['
HOP = {'connection', 'keep-alive', 'proxy-authenticate', 'proxy-authorization',
       'te', 'trailer', 'transfer-encoding', 'upgrade', 'content-length'}

class Handler(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    wbufsize = 65536
    disable_nagle_algorithm = True
    def log_message(self, *_args):
        pass
    def handle_request(self):
        raw = self.rfile.read(int(self.headers.get('Content-Length', '0')))
        headers = {key: value for key, value in self.headers.items()
                   if key.lower() not in HOP | {'host'}}
        connection = http.client.HTTPConnection('127.0.0.1', args.upstream, timeout=60)
        try:
            connection.request(self.command, self.path, body=raw or None, headers=headers)
            response = connection.getresponse()
            body = response.read()
            response_headers = response.getheaders()
            if self.command == 'GET' and urlsplit(self.path).path == '/agents' and response.status == 200:
                value = json.loads(body)
                assert isinstance(value, dict) and value.get('objectType') == 'Person'
                # Common conforming fixture preparation: Kiva emits null for
                # absent optional Person arrays. Omit those absent properties
                # in both branches; the sole branch difference is the URI.
                value = {key: item for key, item in value.items()
                         if not (key in {'name', 'mbox', 'mbox_sha1sum', 'openid', 'account'} and item is None)}
                assert isinstance(value.get('openid', []), list)
                index = len(value.get('openid', []))
                value.setdefault('openid', []).append(added)
                body = json.dumps(value, separators=(',', ':')).encode()
                with args.trace.open('a') as trace:
                    trace.write(json.dumps({'path': self.path, 'mode': args.mode,
                        'added_pointer': '/openid/' + str(index), 'added_value': added,
                        'delivered_response': value}) + '\n')
            self.send_response_only(response.status)
            for key, value in response_headers:
                if key.lower() not in HOP:
                    self.send_header(key, value)
            length = dict((k.lower(), v) for k, v in response_headers).get('content-length')
            self.send_header('Content-Length', length if self.command == 'HEAD' and length else str(len(body)))
            self.end_headers()
            if self.command != 'HEAD':
                self.wfile.write(body)
                self.wfile.flush()
        finally:
            connection.close()
    do_GET = do_HEAD = do_PUT = do_POST = do_DELETE = do_OPTIONS = handle_request

class Server(ThreadingHTTPServer):
    daemon_threads = True
    request_queue_size = 256

Server(('127.0.0.1', args.listen), Handler).serve_forever()
