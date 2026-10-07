#!/usr/bin/env python3
"""Execute bounded subconditions of all 25 LRS review procedures.

Raw HTTP evidence and unexecuted rubric conditions are retained. A procedure
with an unmet fixture or an unexamined subcondition is incomplete, never PASS.
Reference fixture results are design evidence, not target conformance results.
"""
from __future__ import annotations

import argparse
import copy
import datetime as dt
import email.policy
import email.utils
import hashlib
import http.client
import json
import os
import re
import subprocess
import time
import uuid
from email.parser import BytesParser
from pathlib import Path
from urllib.parse import urlencode, urlsplit

import yaml

ROOT = Path(__file__).resolve().parents[2]
IFI = ['mbox', 'mbox_sha1sum', 'openid', 'account']


def stamp(value):
    return dt.datetime.fromisoformat(value.replace('Z', '+00:00'))


def utc():
    return dt.datetime.now(dt.timezone.utc).isoformat().replace('+00:00', 'Z')


def compact(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'))


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ptr(value, pointer):
    for key in pointer.strip('/').split('/') if pointer else []:
        key = key.replace('~1', '/').replace('~0', '~')
        value = value[int(key)] if isinstance(value, list) else value[key]
    return value


def preservation(sent, got, pointer=''):
    """Per-pointer comparison with narrowly named permitted transformations."""
    observations = []
    def record(ok, reason):
        observations.append({'pointer': pointer, 'outcome': 'passed' if ok else 'failed', 'rule': reason, 'submitted': sent, 'returned': got})
    if pointer == '/authority' or pointer == '/stored':
        record(True, 'declared LRS authority/stored overwrite; mapping checked separately')
    elif pointer.endswith('/timestamp') and isinstance(sent, str) and isinstance(got, str):
        try:
            record(got.endswith(('Z', '+00:00')) and abs((stamp(sent)-stamp(got)).total_seconds()) <= .001, 'UTC and permitted precision at three decimal places')
        except ValueError:
            record(False, 'invalid timestamp')
    elif pointer.endswith('/duration') and isinstance(sent, str) and isinstance(got, str):
        if sent == got:
            record(True, 'unchanged duration')
        else:
            observations.append({'pointer':pointer,'outcome':'not-run','rule':'different duration requires independent ISO duration/precision assessment','submitted':sent,'returned':got})
    elif isinstance(sent, dict):
        if isinstance(got, list) and '/contextActivities/' in pointer:
            if len(got) == 1:
                return preservation(sent, got[0], pointer+'/0')
            record(False, 'contextActivities singleton expands to exactly one element')
        elif not isinstance(got, dict):
            record(False, 'object type preservation')
        else:
            for key, value in sent.items():
                sub = pointer+'/'+key.replace('~','~0').replace('/','~1')
                if key not in got:
                    observations.append({'pointer':sub,'outcome':'failed','rule':'submitted property missing','submitted':value})
                else:
                    observations.extend(preservation(value, got[key], sub))
    elif isinstance(sent, list):
        if not isinstance(got, list) or len(sent) != len(got):
            record(False, 'array type and cardinality')
        elif pointer.endswith('/member'):
            remaining = list(got)
            for index, value in enumerate(sent):
                candidate = next((v for v in remaining if any(k in value and value[k]==v.get(k) for k in IFI)), None)
                if candidate is None:
                    observations.append({'pointer':pointer+'/'+str(index),'outcome':'failed','rule':'Group members compared without ordering'})
                else:
                    remaining.remove(candidate)
                    observations.extend(preservation(value,candidate,pointer+'/'+str(index)))
        else:
            for index, value in enumerate(sent):
                observations.extend(preservation(value,got[index],pointer+'/'+str(index)))
    else:
        record(type(sent) is type(got) and sent == got or not isinstance(sent,bool) and not isinstance(got,bool) and isinstance(sent,(int,float)) and isinstance(got,(int,float)) and sent == got, 'submitted scalar value/type')
    return observations


class Executor:
    def __init__(self, args):
        self.args, self.out = args, args.out
        self.fixture = json.loads(args.fixture.read_text())
        for path, digest in self.fixture.get('policy_inputs', {}).items():
            if sha(ROOT/path) != digest:
                raise ValueError('fixture policy changed: '+path)
        self.headers = {'X-Experience-API-Version':'2.0.0','Content-Type':'application/json'}
        self.results, self.records, self.current = {}, [], ''
        self.prefix = 'https://example.org/procedure/'+str(uuid.uuid4())
        self.actor = {'objectType':'Agent','mbox':'mailto:'+str(uuid.uuid4())+'@example.org'}
        self.docs = [('state','/activities/state',{'activityId':self.prefix,'agent':compact(self.actor)},'stateId'),('agent-profile','/agents/profile',{'agent':compact(self.actor)},'profileId'),('activity-profile','/activities/profile',{'activityId':self.prefix},'profileId')]
        self.ordinal=0

    def check(self, name, ok=None, reason='', details=None, advisory=False):
        outcome = 'observed' if advisory else 'not-run' if ok is None else 'passed' if ok else 'failed'
        self.results[self.current]['checks'].append({'id':name,'outcome':outcome,'reason':reason,'details':details})

    def http(self, method, path, query=None, body=None, headers=None, raw=None):
        request_headers = dict(self.headers)
        request_headers.update(headers or {})
        data = raw if raw is not None else compact(body).encode() if body is not None else None
        endpoint=urlsplit(self.args.endpoint)
        target=endpoint.path.rstrip('/')+path+('?' + urlencode(query) if query else '')
        connection=(http.client.HTTPSConnection if endpoint.scheme=='https' else http.client.HTTPConnection)(endpoint.hostname,endpoint.port,timeout=30)
        before=utc()
        connection.request(method,target,body=data,headers=request_headers)
        response=connection.getresponse()
        encoded=response.read()
        after=utc()
        reply={'status':response.status,'headers':{k.lower():v for k,v in response.getheaders()},'body_hex':encoded.hex(),'before':before,'after':after}
        try:reply['json']=json.loads(encoded)
        except (ValueError,UnicodeError):reply['json']=None
        self.ordinal+=1
        evidence={'ordinal':self.ordinal,'procedure':self.current,'request':{'method':method,'path':path,'query':query or {},'headers':{k:('[private]' if k.lower()=='authorization' else v) for k,v in request_headers.items()},'body_hex':(data or b'').hex()},'response':reply}
        with (self.out/'http.jsonl').open('a') as f:f.write(compact(evidence)+'\n')
        connection.close()
        return reply

    def control(self, **kwargs):
        if not self.fixture.get('clock_control'):
            raise ValueError('independent clock fixture not declared')
        self.http('POST','/_control',body=kwargs)

    def statement(self, **extra):
        return {'id':str(uuid.uuid4()),'actor':copy.deepcopy(self.actor),'verb':{'id':self.prefix+'/'+str(uuid.uuid4())},'object':{'id':self.prefix+'/'+str(uuid.uuid4())},**extra}

    def store(self, body, headers=None):
        result=self.http('PUT','/statements',{'statementId':body['id']},body,headers)
        self.check('setup-write-'+body['id'],result['status']==204,'valid fixture Statement must store')
        return result

    def read(self, body, **query):
        return self.http('GET','/statements',{'statementId':body['id'],**query})

    def conflict(self):
        known={'Check current state and use If-Match with the current ETag.':True,'document already exists':False,'Conflict':False}
        for family,path,scope,key in self.docs:
            q={**scope,key:'conflict'}
            self.http('PUT',path,q,{'v':1})
            response=self.http('PUT',path,q,{'v':2})
            value=response['json']
            message=value.get('message',value.get('error')) if isinstance(value,dict) else bytes.fromhex(response['body_hex']).decode(errors='replace')
            self.check(family+'-409',response['status']==409,'unconditional existing document PUT')
            self.check(family+'-explanation',known.get(message),'Semantic assessment of exact English messages: full message directs checking current state and retrying with current ETag in If-Match; generic conflict/existence does neither. Other text requires semantic review.',{'language':response['headers'].get('content-language','und'),'message':message})

    def consistency(self):
        if not self.fixture.get('clock_control'):
            self.check('delayed-visibility-safety',None,'Target has no declared clock/visibility control. A fixed old watermark alone does not violate a numerical recency requirement. No numerical bound is specified.')
            return
        self.control(clock='1969-12-31T23:59:59Z',hide_new=True)
        body=self.statement();self.store(body)
        self.control(clock='1970-01-01T00:00:01Z')
        for label,method,path,query in [('success','GET','/statements',{'verb':body['verb']['id']}),('error','GET','/statements',{'statementId':str(uuid.uuid4())}),('head','HEAD','/statements',{'verb':body['verb']['id']})]:
            reply=self.http(method,path,query)
            try:ok=stamp(reply['headers']['x-experience-api-consistent-through']) <= stamp('1969-12-31T23:59:59Z')
            except (KeyError,ValueError):ok=False
            self.check('hidden-record-'+label,ok,'Watermark must not cross known stored time of invisible Statement')
        self.control(hide_new=False,release=True,clock='1970-01-01T00:00:02Z')
        reply=self.read(body)
        self.check('released-retrieval',reply['status']==200 and reply['json'].get('id')==body['id'])

    def documents(self):
        for family,path,scope,key in self.docs:
            q={**scope,key:'clock'}
            if self.fixture.get('clock_control'):self.control(clock='2026-10-06T00:00:01.250Z')
            created=self.http('PUT',path,q,{'v':1})
            first=self.http('GET',path,q)
            if self.fixture.get('clock_control'):self.control(clock='2026-10-06T00:00:03.750Z')
            else:time.sleep(1.1)
            updated=self.http('PUT',path,q,{'v':2},{'If-Match':first['headers'].get('etag','*')})
            second=self.http('GET',path,q)
            self.check(family+'-write-read',created['status']==updated['status']==204 and first['status']==second['status']==200)
            try:
                lm=email.utils.parsedate_to_datetime(second['headers']['last-modified'])
                if self.fixture.get('clock_control'):ok=lm==stamp('2026-10-06T00:00:03Z')
                elif self.fixture.get('host_clock_shared'):ok=stamp(updated['before']).replace(microsecond=0)<=lm<=stamp(updated['after']).replace(microsecond=0)
                else:ok=None
            except (KeyError,ValueError):ok=False
            self.check(family+'-last-modified',ok,'Last-Modified matches the controlled time or declared same-host UTC write interval, at HTTP-date precision')
            self.check(family+'-modified-moves',first['headers'].get('last-modified')!=second['headers'].get('last-modified'),'Clock fixture crosses an HTTP-date second')
            lower='2026-10-06T00:00:03.749Z' if self.fixture.get('clock_control') else updated['before']
            upper='2026-10-06T00:00:03.751Z' if self.fixture.get('clock_control') else updated['after']
            for label,since,present in [('before',lower,True),('after',upper,False)]:
                reply=self.http('GET',path,{**scope,'since':since})
                self.check(family+'-since-'+label,reply['status']==200 and isinstance(reply['json'],list) and ('clock' in reply['json'])==present,'Known independent bounding timestamps; since is exclusive')
            if self.fixture.get('clock_control'):
                reply=self.http('GET',path,{**scope,'since':'2026-10-06T00:00:03.750Z'})
                self.check(family+'-since-equal',reply['status']==200 and 'clock' not in reply['json'],'Equal timestamp is controlled independently of Last-Modified')
            else:self.check(family+'-since-equal',None,'Cannot control exact subsecond updated time; HTTP-date is insufficient')
        _,path,scope,key=self.docs[0]
        sibling={**scope,'activityId':self.prefix+'/other-scope',key:'preserved'}
        self.http('PUT',path,sibling,{'v':'sibling'})
        self.http('DELETE',path,scope)
        deleted=self.http('GET',path,{**scope,key:'clock'})
        kept=self.http('GET',path,sibling)
        self.check('state-delete-scope',deleted['status']==404 and kept['status']==200 and kept['json']=={'v':'sibling'})

    def identifier_syntax(self, got):
        values = []
        for key in IFI:
            entries = got.get(key, [])
            if not isinstance(entries, list):
                continue
            for item in entries:
                value = item.get('homePage') if key == 'account' and isinstance(item, dict) else item
                if key != 'mbox_sha1sum' and isinstance(value, str):
                    pair = (value, key == 'openid')
                    if pair not in values:
                        values.append(pair)
        path = self.out / ('identifier-check-' + str(self.ordinal) + '.json')
        path.write_text(compact([{'value': value, 'ascii_only': ascii_only} for value, ascii_only in values]))
        run = subprocess.run([str(self.args.runner), '--check-identifiers', str(path)], capture_output=True, text=True, check=True)
        verdicts = json.loads(run.stdout)['valid']
        if len(verdicts) != len(values) or any(type(v) is not bool for v in verdicts):
            raise ValueError('Invalid grammar-check response')
        (path.with_suffix('.result.json')).write_text(run.stdout)
        return dict(zip(values, verdicts))

    def person(self):
        examples={'mbox':'mailto:unknown-'+str(uuid.uuid4())+'@example.org','mbox_sha1sum':'0123456789abcdef0123456789abcdef01234567','openid':self.prefix+'/openid','account':{'homePage':self.prefix+'/account','name':'learner'}}
        for field,value in examples.items():
            reply=self.http('GET','/agents',{'agent':compact({field:value})})
            got=reply['json']
            self.check(field+'-queried',reply['status']==200 and isinstance(got,dict) and value in got.get(field,[]),'Unknown Agents must return the queried IFI')
            if not isinstance(got,dict):continue
            syntax = self.identifier_syntax(got)
            for key in IFI:
                entries=got.get(key,[])
                self.check(field+'-'+key+'-array',isinstance(entries,list))
                if not isinstance(entries,list):continue
                for index,item in enumerate(entries):
                    valid=isinstance(item,str) and syntax.get((item,False),False) and item.split(':')[0].lower()=='mailto' and bool(item.split(':',1)[1]) if key=='mbox' else isinstance(item,str) and re.fullmatch(r'[0-9a-fA-F]{40}',item) is not None if key=='mbox_sha1sum' else isinstance(item,str) and syntax.get((item,True),False) if key=='openid' else isinstance(item,dict) and isinstance(item.get('homePage'),str) and syntax.get((item['homePage'],False),False) and isinstance(item.get('name'),str)
                    self.check(field+'-'+key+'-'+str(index),valid,'Every enrichment entry uses the shared RFC3986/3987 syntax check and Agent field type')
        self.check('full-external-grammar',None,'Generic URI/IRI grammar now checked; mailto-specific syntax, IRL semantics and full external-class closure remain open')

    def metadata(self, language=False):
        if not self.fixture.get('canonical_submitted_fixture'):
            self.check('declared-inventory',None,'No declared maintained metadata inventory; submitted definitions alone cannot establish it')
            return
        definition={'name':{'en':'English','ja-JP':'日本語'},'description':{'en':'Only English'},'type':self.prefix+'/type','interactionType':'choice','choices':[{'id':'a','description':{'fr':'Français','en':'English choice'}}]}
        body=self.statement(object={'id':self.prefix+'/known-activity','definition':definition})
        if language:
            body['context']={'contextActivities':{'category':[{'id':self.prefix+'/category','definition':{'name':{'fr':'Catégorie','en':'Category'}}}]}}
        self.store(body)
        if not language:
            for name,activity,expected in [('known',body['object']['id'],definition),('unknown',self.prefix+'/unknown-activity',None)]:
                reply=self.http('GET','/activities',{'activityId':activity})
                got=reply['json']
                self.check(name+'-activity-id',reply['status']==200 and isinstance(got,dict) and got.get('id')==activity)
                if expected is not None:
                    self.check('available-definition',isinstance(got,dict) and got.get('definition')==expected,'Explicit fixture policy maintains all supplied properties; inventory recorded before retrieval',{'inventory':expected,'policy':self.fixture['canonical_submitted_fixture']})
            return
        maps={'/object/definition/name':definition['name'],'/object/definition/description':definition['description'],'/object/definition/choices/0/description':definition['choices'][0]['description'],'/context/contextActivities/category/0/definition/name':body['context']['contextActivities']['category'][0]['definition']['name']}
        for label,header,preferred in [('weighted','en;q=0.1, ja-JP;q=0.9','ja-JP'),('range','ja;q=1, en;q=0.5','ja-JP'),('wildcard','*',None),('unavailable','de',None),('absent',None,None)]:
            response=self.http('GET','/statements',{'statementId':body['id'],'format':'canonical'},headers={'Accept-Language':header} if header else {})
            for pointer,inventory in maps.items():
                try:
                    got=ptr(response['json'],pointer)
                    expected=preferred if preferred in inventory else 'en' if label in ['weighted','range'] and 'en' in inventory else None
                    ok=isinstance(got,dict) and len(got)==1 and all(k in inventory and inventory[k]==v for k,v in got.items()) and (expected is None or expected in got)
                except (KeyError,TypeError,IndexError):ok=False
                self.check(label+pointer,ok,'Each maintained Activity language map independently; unavailable preference permits one available language',{'inventory':inventory})

    def versions(self):
        for version in ['1.0.3','1.1.0','2.1.0']:
            reply=self.http('GET','/about',headers={'X-Experience-API-Version':version})
            text=bytes.fromhex(reply['body_hex']).decode(errors='replace')
            if reply['status']==400:
                # Assessment of actual exact responses is retained for semantic review.
                known=reply['json'].get('message',reply['json'].get('error','')) if isinstance(reply['json'],dict) else text
                valid=known in ['unsupported xAPI version','xAPI version is unsupported','unsupported X-Experience-API-Version','X-Experience-API-Version must be 2.0 or 2.0.0'] or known=='Unsupported request version '+version
                self.check(version+'-rejection-explanation',True if valid else None,'Known exact message describes unsupported version; other response needs semantic review',{'message':known})
            elif version.startswith('2.'):
                self.check(version+'-rejection',False,'2.1.0 or later must reject')
            else:self.check(version+'-prior-conformance',None,'Accepted older-version request requires that routed version\'s complete Suite; status alone is insufficient',{'status':reply['status']})

    def precision(self):
        for nested in [False,True]:
            for timestamp in ['2025-01-01T09:00:00.123+09:00','2025-01-01T09:00:00.123456+09:00']:
                body=self.statement(timestamp=timestamp)
                if nested:
                    sub=copy.deepcopy(body);sub.pop('id');body=self.statement(object={'objectType':'SubStatement',**sub})
                self.store(body);reply=self.read(body)
                try:
                    got=ptr(reply['json'],'/object/timestamp' if nested else '/timestamp')
                    ok=got.endswith(('Z','+00:00')) and abs((stamp(got)-stamp(timestamp)).total_seconds())<=.001
                except (KeyError,TypeError,ValueError):ok=False
                self.check(('sub-' if nested else 'top-')+timestamp,ok,'At least millisecond instant precision and UTC, permitted higher precision rounding/truncation')

    def duplicate(self):
        body=self.statement(object={'id':self.prefix+'/immutable','definition':{'name':{'en':'original'}}})
        self.store(body)
        before=self.read(body)
        activity_before=self.http('GET','/activities',{'activityId':body['object']['id']})
        person_before=self.http('GET','/agents',{'agent':compact(body['actor'])})
        for method in ['PUT','POST']:
            changed=copy.deepcopy(body);changed['object']['definition']['name']['en']='changed'
            response=self.http(method,'/statements',{'statementId':body['id']} if method=='PUT' else {},changed)
            self.check(method+'-allowed-status',response['status'] in ([204,409] if method=='PUT' else [200,204,409]))
            after=self.read(body)
            activity_after=self.http('GET','/activities',{'activityId':body['object']['id']})
            person_after=self.http('GET','/agents',{'agent':compact(body['actor'])})
            self.check(method+'-all-object-preservation',before['json']==after['json'] and activity_before['json']==activity_after['json'] and person_before['json']==person_after['json'],'Duplicate must not change stored Statement, Activity or Person')
        self.check('all-excluded-property-and-delayed-visibility-variants',None,'Finite immutable mutation executed; every excluded-property duplicate and delayed fixture remains open')

    def voiding(self):
        a=self.statement();self.store(a)
        b=self.statement(verb={'id':'http://adlnet.gov/expapi/verbs/voided'},object={'objectType':'StatementRef','id':a['id']});self.store(b)
        c=self.statement(verb={'id':'http://adlnet.gov/expapi/verbs/voided'},object={'objectType':'StatementRef','id':b['id']})
        response=self.http('PUT','/statements',{'statementId':c['id']},c)
        self.check('c-allowed-status',response['status'] in [204,400,403,409])
        for name,body,visible in [('a',a,False),('b',b,True),('c',c,response['status']==204)]:
            single=self.read(body)
            listing=self.http('GET','/statements',{'verb':body['verb']['id']})
            ids=[s['id'] for s in listing.get('json',{}).get('statements',[])] if isinstance(listing['json'],dict) else []
            self.check(name+'-retrievability',single['status']==(200 if visible else 404) and (body['id'] in ids)==visible,'A remains voided; B remains returnable; C retrieval follows actual acceptance')

    def recursive_formats(self):
        from lrs_projection_responses import format_cases
        vectors=[c for c in format_cases() if c['id'].endswith('-ids') and any(x.get('kind')=='json-shape' for s in c['steps'] for x in s['expect'].get('checks',[]))]
        for case in vectors:
            body=copy.deepcopy(case['steps'][0]['request']['body']);body['id']=str(uuid.uuid4())
            response=self.store(body)
            if response['status']!=204:continue
            for format_name in ['exact','ids','canonical']:
                reply=self.read(body,format=format_name)
                if format_name=='ids':
                    for check in next(s['expect']['checks'] for s in case['steps'] if any(c.get('kind')=='json-shape' for c in s['expect'].get('checks',[]))):
                        if check.get('kind')!='json-shape' or check['shape'].get('additional_properties') is not False:continue
                        try:got=ptr(reply['json'],check.get('pointer',''));allowed=set(check['shape'].get('properties',{}));required=set(check['shape'].get('required',[]));ok=isinstance(got,dict) and required<=got.keys() and got.keys()<=allowed
                        except (KeyError,TypeError):ok=False
                        self.check(case['id']+'-ids'+check.get('pointer',''),ok,'Minimum recursive identifying fields; member inner checks remain in automatic cases')
                else:
                    obs=preservation(body,reply['json']) if format_name=='exact' else preservation(body['actor'],reply['json'].get('actor'),'/actor')+preservation(body.get('context',{}),reply['json'].get('context',{}),'/context') if format_name=='canonical' else []
                    # Canonical Context Activities may change; only Agent/Group portions are compared below.
                    if format_name=='canonical':obs=[o for o in obs if '/contextActivities/' not in o['pointer']]
                    self.check(case['id']+'-'+format_name,all(o['outcome']=='passed' for o in obs),details=obs)
        self.check('all-ids-nested-identifying-values',None,'This direct procedure checks legal keys; automatic json-shape controls supply inner member assertions; full combined rubric is not closed')

    def tables(self):
        files=[ROOT/'tests/conformance/http/generated/tables.yaml',ROOT/'tests/conformance/http/generated/values.yaml']
        tested=0
        for file in files:
            if not file.exists():continue
            for case in yaml.load(file.read_text(),Loader=yaml.CSafeLoader)['cases']:
                first=case.get('steps',[case])[0]
                req=first['request'];exp=first['expect']
                if req['path']!='/statements' or not isinstance(req.get('body'),dict) or exp.get('status') not in [200,204]:continue
                body=copy.deepcopy(req['body']);identifier=str(uuid.uuid4());body['id']=identifier
                query={'statementId':identifier} if req['method']=='PUT' else {}
                reply=self.http(req['method'],req['path'],query,body)
                self.check(case['id']+'-acceptance',reply['status'] in [200,204],'Required acceptance vector')
                if reply['status'] not in [200,204]:continue
                returned=self.read(body)['json'];obs=preservation(body,returned)
                if isinstance(returned,dict):
                    for key in ['id','stored','timestamp','authority','version']:
                        valid=key in returned and (isinstance(returned[key],dict) if key=='authority' else isinstance(returned[key],str))
                        if key in ['stored','timestamp'] and valid:
                            try:stamp(returned[key])
                            except ValueError:valid=False
                        if key=='id' and valid:
                            try:uuid.UUID(returned[key])
                            except ValueError:valid=False
                        obs.append({'pointer':'/'+key,'outcome':'passed' if valid else 'failed','rule':'required returned field type'})
                with (self.out/'preservation.jsonl').open('a') as f:f.write(compact({'case':case['id'],'observations':obs})+'\n')
                self.check(case['id']+'-preservation',False if any(o['outcome']=='failed' for o in obs) else None if any(o['outcome']=='not-run' for o in obs) else True,'Every submitted pointer walked; see preservation.jsonl')
                tested+=1
        self.check('coverage',True,details={'accepted_vectors_read':tested})
        self.check('omission-post-batches-two-attachment-raw-parts',None,'Omissions, all repeated POST batches, and both distinct attachment multipart fixtures remain an explicit separate rubric gap')

    def http_rfc(self):
        for family,path,scope,key in self.docs:
            q={**scope,key:'http'}
            self.http('PUT',path,q,{'text':'UTF-8 日本語'})
            get=self.http('GET',path,q);head=self.http('HEAD',path,q,headers={'x-experience-api-version':'2.0.0'})
            self.check(family+'-head',get['status']==head['status']==200 and not head['body_hex'] and all(get['headers'].get(k)==head['headers'].get(k) for k in ['content-type','etag','last-modified']))
            stale=self.http('PUT',path,q,{'text':'changed'},{'iF-mAtCh':'"stale"'})
            self.check(family+'-case-insensitive-if-match',stale['status']==412)
        self.check('complete-rfc-header-context-framing-grammar',None,'Finite HEAD and header casing probes do not discharge every HTTP RFC requirement')

    def mime(self):
        data=b'\x00binary\xff\r\n--boundary-near-but-not-delimiterXX\r\nend'
        digest=hashlib.sha256(data).hexdigest()
        for label,boundary,quoted in [('quoted','boundary-near-but-not-delimiter',True),('ordinary','normal-boundary',False)]:
            body=self.statement(attachments=[{'usageType':self.prefix+'/attachment','display':{'en':'binary'},'contentType':'application/octet-stream','length':len(data),'sha2':digest}])
            raw=b'preamble\r\n--'+boundary.encode()+b'\r\nContent-Type: application/json\r\n\r\n'+compact(body).encode()+b'\r\n--'+boundary.encode()+b'\r\ncOnTeNt-TyPe: application/octet-stream\r\nX-Experience-API-Hash: '+digest.encode()+b'\r\nContent-Transfer-Encoding: binary\r\n\r\n'+data+b'\r\n--'+boundary.encode()+b'--\r\nepilogue'
            media='multipart/mixed; boundary='+('"'+boundary+'"' if quoted else boundary)
            parsed=BytesParser(policy=email.policy.default).parsebytes(('Content-Type: '+media+'\r\nMIME-Version: 1.0\r\n\r\n').encode()+raw)
            parts=list(parsed.iter_parts())
            self.check(label+'-independent-input-parser',len(parts)==2 and parts[1].get_payload(decode=True)==data)
            response=self.http('PUT','/statements',{'statementId':body['id']},headers={'Content-Type':media},raw=raw)
            self.check(label+'-accepted',response['status']==204)
            if response['status']==204:
                returned=self.http('GET','/statements',{'statementId':body['id'],'attachments':'true'})
                message=BytesParser(policy=email.policy.default).parsebytes(('Content-Type: '+returned['headers'].get('content-type','')+'\r\nMIME-Version: 1.0\r\n\r\n').encode()+bytes.fromhex(returned['body_hex']))
                output=list(message.iter_parts()) if message.is_multipart() else []
                self.check(label+'-returned-binary-hash',len(output)==2 and hashlib.sha256(output[1].get_payload(decode=True)).hexdigest()==digest and output[1].get_payload(decode=True)==data)
        self.check('legal-folding-transfer-variations-full-rfc',None,'These two witnesses do not close all MIME framing/transport classes')

    def limits(self):
        if self.fixture.get('size_limit'):
            response=self.http('POST','/statements',body=self.statement(result={'response':'x'*(self.fixture['size_limit']+1)}))
            self.check('declared-size-overrun',response['status']==413)
        else:self.check('declared-size-overrun',None,'No numerical limit fixture declared')
        if self.fixture.get('rate_path'):
            time.sleep(self.fixture['rate_window_seconds']+.1)
            trigger=self.fixture['rate_trigger_request_count']
            statuses=[self.http('GET',self.fixture['rate_path'])['status'] for _ in range(trigger)]
            self.check('declared-rate-overrun',statuses==[200]*(trigger-1)+[429],'Declared trigger includes the denied request; begin after one idle window',details=statuses)
        else:self.check('declared-rate-overrun',None,'No rate fixture declared')
        for path in ['/statements','/about','/agents','/activities',*[d[1] for d in self.docs]]:
            self.check(path+'-unknown',self.http('GET',path,{'unknownProcedureParameter':'value'})['status']==400)
        self.check('all-size-doc-attachment-extension-contexts',None,'Only declared Statement body and rate limits exercised; no invented document or attachment limit')

    def authorization(self):
        policy=self.fixture.get('authorization_policy')
        self.check('decision-boundary',True if policy else None,'Fixture information from configured policy, not a normative authority',policy)
        invalid=os.environ.get('KIVA_CONFORMANCE_INVALID_TOKEN');reader=os.environ.get('KIVA_CONFORMANCE_READER_TOKEN');writer=os.environ.get('KIVA_CONFORMANCE_WRITER_A_TOKEN')
        if not all([invalid,reader,writer]):self.check('credential-fixtures',None,'Refused/permitted/operation-denied credential fixtures unavailable');return
        response=self.http('GET','/about',headers={'Authorization':'Bearer '+invalid})
        self.check('refused-401',response['status']==401)
        response=self.http('PUT','/statements',{'statementId':str(uuid.uuid4())},self.statement(),{'Authorization':'Bearer '+reader})
        self.check('denied-write-403',response['status']==403)
        body=self.statement();self.store(body,{'Authorization':'Bearer '+writer})

    def authority(self):
        policy=self.fixture.get('authority_policy')
        self.check('declared-overwrite-policy',True if policy else None,details=policy)
        authorities=[]
        for name,token in [('anonymous',None),('writer',os.environ.get('KIVA_CONFORMANCE_WRITER_A_TOKEN'))]:
            if name=='writer' and not token:self.check(name+'-credential',None,'Second identity unavailable');continue
            current=[]
            for _ in range(2):
                body=self.statement(authority={'mbox':'mailto:submitted@example.org'})
                self.store(body,{'Authorization':'Bearer '+token} if token else {})
                reply=self.read(body)
                value=reply['json'].get('authority') if isinstance(reply['json'],dict) else None
                self.check(name+'-authority-present-'+body['id'],isinstance(value,dict) and any(k in value for k in IFI))
                current.append(value)
            self.check(name+'-deterministic',current[0]==current[1])
            authorities.append(current[0])
        self.check('distinct-credential-identities',len(authorities)==2 and authorities[0]!=authorities[1] if len(authorities)==2 else None,'Declared mapping yields two distinct credential identities')

    def related_authority(self):
        run=subprocess.run([os.sys.executable,str(ROOT/'scripts/ci/probe_lrs_authority_filter.py'),'--endpoint',self.args.endpoint,'--runner',str(self.args.runner),'--output',str(self.out/'authority-filter')],capture_output=True,text=True)
        (self.out/'authority-filter.log').write_text(run.stdout+run.stderr)
        self.check('actual-returned-authority-true-default-false',None if run.returncode==2 else run.returncode==0,'Actual returned authority and all identifiable Group members; portable runner evidence retained',{'exit_code':run.returncode})

    def languages(self):
        for tag,valid in [('en',True),('zh-Hant-CN',True),('i-klingon',True),('x-private',True),('abcdefgh',True),('abcdefghi',False),('en-abcdefghi',False)]:
            for nested in [False,True]:
                body=self.statement(object={'id':self.prefix+'/lang/'+str(uuid.uuid4()),'definition':{'name':{tag:'plain text <b>'}}})
                if nested:
                    sub=copy.deepcopy(body);sub.pop('id');body=self.statement(object={'objectType':'SubStatement',**sub})
                response=self.http('PUT','/statements',{'statementId':body['id']},body)
                self.check(tag+('-sub' if nested else '-top'),response['status'] in [204,400] if valid and tag in ['i-klingon','abcdefgh','x-private'] else response['status']==(204 if valid else 400),'Minimum token lengths mandatory; optional registry validation may reject unregistered well-formed values')
        self.check('full-rfc-every-language-map-location',None,'Finite independent RFC5646 syntax/token-length vectors; external grammar and every placement remain open')

    def references(self):
        a=self.statement();self.store(a)
        b=self.statement(object={'objectType':'StatementRef','id':a['id']});self.store(b)
        c=self.statement(object={'objectType':'StatementRef','id':b['id']});self.store(c)
        for field,value in [('verb',a['verb']['id']),('activity',a['object']['id']),('agent',compact(a['actor']))]:
            reply=self.http('GET','/statements',{field:value})
            ids=[v['id'] for v in reply['json'].get('statements',[])] if isinstance(reply['json'],dict) else []
            self.check(field+'-recursive-inheritance',{a['id'],b['id'],c['id']}<=set(ids))
        context=self.statement(context={'statement':{'objectType':'StatementRef','id':a['id']}});self.store(context)
        reply=self.http('GET','/statements',{'verb':a['verb']['id']})
        self.check('context-does-not-inherit',context['id'] not in [v['id'] for v in reply['json'].get('statements',[])])
        self.check('controlled-time-registration-limit-cycles-voided-targets',None,'Remaining reference combinations and cycle fixtures not yet controlled')

    def interaction(self):
        policy=self.fixture.get('interaction_policy')
        self.check('declared-policy',True if policy else None,details=policy)
        for kind,statuses in [('invalid-type',[400]),('choice',[204,400]),('performance',[204,400])]:
            definition={'interactionType':kind,'correctResponsesPattern':['{case_matters=true}arbitrary']}
            if kind=='choice':definition['choices']=[{'id':'a','description':{'en':'A'}}]
            if kind=='performance':definition['steps']=[{'id':'a','description':{'en':'A'}}]
            body=self.statement(object={'id':self.prefix+'/'+kind,'definition':definition})
            response=self.http('PUT','/statements',{'statementId':body['id']},body)
            self.check(kind+'-strength',response['status'] in statuses)
        self.check('all-components-patterns-characterstring-parameters',None,'Optional acceptance is permitted; exhaustive chosen validation policy not yet reconciled')

    def advisory_metadata(self):
        self.check('declared-network-metadata-policy',reason='Policy observation only; no mandatory network fetch',details=self.fixture.get('resolvable_metadata_policy','undeclared'),advisory=True)
        self.check('controlled-hosted-metadata-observation',None,'No controlled external metadata host supplied; optional behavior remains unobserved')

    def rejection(self):
        body=self.statement(timestamp=utc())
        raw=compact(dict(reversed(list(body.items())))).encode()
        reply=self.http('PUT','/statements',{'statementId':body['id']},raw=raw)
        self.check('reordered-json',reply['status']==204)
        missing=self.statement(verb={'id':'http://adlnet.gov/expapi/verbs/voided'},object={'objectType':'StatementRef','id':str(uuid.uuid4())})
        self.store(missing)
        self.check('future-margin-and-batch-advisory',None,'No declared future clock margin or complete whole-batch policy witness; no invented numeric margin')

    def cursor(self):
        self.check('metadata-and-pagination-policy',reason='Allowed implementation choice, not a mandatory alternative',details=self.fixture.get('cursor_policy','undeclared'),advisory=True)
        a=self.statement();self.store(a)
        self.http('GET','/statements',{'limit':'1'})
        b=self.statement();self.store(b)
        self.check('cursor-mutation-observation',None,'Initial page recorded; concurrent cursor replay/voiding and authorized metadata choices remain unobserved')

    def iri(self):
        policy=self.fixture.get('iri_strategy')
        self.check('declared-allowed-strategy',True if policy in ['simple','syntax-normalization','simple-and-syntax-normalization'] else None,'RFC3987 allowed strategy declaration',policy)
        for suffix in ['路径','%70ath','Path','path','a/../path']:
            body=self.statement(object={'id':self.prefix+'/'+suffix})
            self.store(body);reply=self.read(body)
            self.check(suffix+'-received-identity',reply['json'].get('object',{}).get('id')==body['object']['id'])
        self.check('normalization-comparison-and-default-port',None,'Accepted IRI preservation does not establish equality strategy; external grammar/equality classes still open')

    def execute(self):
        catalog=yaml.safe_load((ROOT/'spec/conformance/lrs-review-tests.yaml').read_text())['tests']
        functions=[self.conflict,self.consistency,self.http_rfc,self.mime,self.limits,self.duplicate,self.recursive_formats,self.references,lambda:self.metadata(True),self.documents,self.person,self.metadata,self.versions,self.authorization,self.interaction,self.advisory_metadata,self.rejection,self.authority,self.cursor,self.iri,self.languages,self.precision,self.tables,self.related_authority,self.voiding]
        for spec,function in zip(catalog,functions,strict=True):
            name=spec['id']
            if self.args.procedure and name not in self.args.procedure:continue
            self.current=name
            self.results[name]={'level':spec['level'],'checks':[],'rubric_sha256':hashlib.sha256(compact(spec).encode()).hexdigest()}
            try:function()
            except Exception as error:self.check('execution-error',False,str(error))
            outcomes={c['outcome'] for c in self.results[name]['checks']}
            self.results[name]['outcome']='failed' if 'failed' in outcomes else 'incomplete' if 'not-run' in outcomes else 'observed' if spec['level']=='advisory' else 'passed'
            print(name,self.results[name]['outcome'],flush=True)
        summary={'schema_version':1,'axis':self.fixture['axis'],'scope':'Executed finite rubric subconditions; unexecuted subconditions retained. No whole-standard closure.','target':self.args.endpoint,'fixture':self.fixture,'input_sha256':{str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p):sha(p) for p in [Path(__file__),self.args.fixture,ROOT/'spec/conformance/lrs-review-tests.yaml',self.args.runner]},'procedures':self.results}
        (self.out/'summary.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2)+'\n')
        return 1 if any(v['outcome']=='failed' for v in self.results.values()) else 2 if any(v['outcome']=='incomplete' for v in self.results.values()) else 0


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint',required=True)
    parser.add_argument('--runner',required=True,type=Path)
    parser.add_argument('--fixture',required=True,type=Path)
    parser.add_argument('--out',required=True,type=Path)
    parser.add_argument('--procedure',action='append')
    args=parser.parse_args()
    args.out.mkdir(parents=True,exist_ok=False)
    return Executor(args).execute()


if __name__=='__main__':
    raise SystemExit(main())
