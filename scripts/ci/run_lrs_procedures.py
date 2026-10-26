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
from urllib.parse import urlencode, urlsplit, urljoin

import yaml

from lrs_procedure_definitions import check_level, load_procedures

ROOT = Path(__file__).resolve().parents[2]
IFI = ['mbox', 'mbox_sha1sum', 'openid', 'account']


def host_stamp(value):
    """Only this executor's own utc() clock samples, never target values."""
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


from lrs_preservation import (preservation, timestamp_preserved, timestamp_ns, timestamp_pair, PrimitiveUnavailable,
    statement_response_headers, agent_identity_equal, snapshot_equal, all_decisions,
    load_json_members, iri_reference_valid, mailbox_structure, agent_validity, canonical_statement_id, standard_uuid)


def response_http_date(value):
    """Validate the LRS sender's RFC2616 3.3.1 rfc1123-date before conversion.

    This checks generated response headers, not the recipient's broader duty
    to accept obsolete HTTP-date forms. Mail-date parsers silently supply
    missing seconds, accept other zones, and reinterpret small four-digit years.
    """
    weekdays=('Mon','Tue','Wed','Thu','Fri','Sat','Sun')
    months=('Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec')
    pattern=(r'('+'|'.join(weekdays)+r'), ([0-9]{2}) ('+'|'.join(months)
             +r') ([0-9]{4}) ([0-9]{2}):([0-9]{2}):([0-9]{2}) GMT')
    # RFC2616 4.2 excludes outer field LWS from field-content. Internal date
    # spacing remains exact; bare line breaks and trailing comments fail.
    padding=r'(?:[ \t]|\r\n(?=[ \t]))*'
    match=re.fullmatch(padding+pattern+padding,value) if isinstance(value,str) else None
    if match is None:
        raise ValueError('response HTTP-date must use complete rfc1123-date syntax')
    weekday,day,month,year,hour,minute,second=match.groups()
    year=int(year)
    # datetime validates month/day and 00:00:00..23:59:59. A 400-year cycle
    # checks known year-zero calendar/weekday defects before provider deferral.
    parsed=dt.datetime(year or 400,months.index(month)+1,int(day),int(hour),int(minute),int(second),tzinfo=dt.timezone.utc)
    if weekdays[parsed.weekday()]!=weekday:
        raise ValueError('response HTTP-date weekday disagrees with its date')
    if year==0:
        raise PrimitiveUnavailable('HTTP-date year-zero instant needs a calendar provider')
    return parsed


class EvidenceUnavailable(RuntimeError):
    """Evidence acquisition/capability failure, not a target violation."""


class Reply(dict):
    """An interrupted body is never presented as an empty or malformed body."""
    def __getitem__(self, key):
        if key in ('json', 'body_hex') and dict.get(self, 'incomplete_body'):
            raise EvidenceUnavailable('Response body transport incomplete; observed checks retained')
        return super().__getitem__(key)

    def get(self, key, default=None):
        if key in self:
            return self[key]
        return default


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
        self.visible_statements={}
        self.known_statement_ids=set()
        self.voiding_statement_ids=set()
        self.voided_statement_ids=set()

    def check(self, name, ok=None, reason='', details=None):
        level = check_level(getattr(self, 'current_spec', {}), name)
        outcome = 'observed' if level == 'advisory' else 'not-run' if ok is None else 'passed' if ok else 'failed'
        self.results[self.current]['checks'].append({'id':name,'level':level,'outcome':outcome,'reason':reason,'details':details})

    def http(self, method, path, query=None, body=None, headers=None, raw=None, continuation=None, retain_partial=False, expected_statuses=None, continuation_base=None):
        request_headers = dict(self.headers)
        request_headers.update(headers or {})
        data = raw if raw is not None else compact(body).encode() if body is not None else None
        endpoint=urlsplit(self.args.endpoint)
        dest=endpoint._replace(path=endpoint.path.rstrip('/')+path,
                               query=urlencode(query) if query else '',fragment='')
        if continuation is not None:
            if not iri_reference_valid(continuation):
                raise ValueError('malformed continuation IRI reference')
            if continuation_base is None:
                raise ValueError('continuation source response URL unavailable')
            dest=urlsplit(urljoin(continuation_base,continuation))
            origin=lambda u:(u.scheme,u.hostname,u.port or (443 if u.scheme=='https' else 80))
            if origin(dest)!=origin(endpoint):
                raise ValueError('continuation needs separate destination/credential capability')
        target=(dest.path or '/')+('?' + dest.query if dest.query else '')
        response_url=dest._replace(fragment='').geturl()
        connection=(http.client.HTTPSConnection if endpoint.scheme=='https' else http.client.HTTPConnection)(endpoint.hostname,endpoint.port,timeout=30)
        before=utc()
        connection.request(method,target,body=data,headers=request_headers)
        response=connection.getresponse()
        response_headers={k.lower():v for k,v in response.getheaders()}
        label='response-'+str(self.ordinal+1)
        if expected_statuses is not None:
            self.check(label+'-status',response.status in expected_statuses,
                'Declared call-specific status oracle, recorded before body read or later requests',
                {'actual':response.status,'allowed':list(expected_statuses)})
        header_result=True
        xapi_paths={'/statements','/about','/agents','/activities',*[d[1] for d in self.docs]}
        if path in xapi_paths or continuation is not None:
            label='response-'+str(self.ordinal+1)
            routed_legacy=request_headers.get('X-Experience-API-Version','').startswith('1.') and response.status < 400
            version_value=response_headers.get('x-experience-api-version')
            version=False if version_value is None else None if routed_legacy else version_value=='2.0.0'
            self.check(label+'-version-header',version,'Mandatory on each observed xAPI response, including errors/setup')
            header_result=version
            if path=='/statements' or continuation is not None:
                _,consistent=statement_response_headers(response_headers)
                self.check(label+'-consistent-through-header',consistent)
                header_result=all_decisions([version,consistent])
        incomplete_body=False
        try:encoded=response.read()
        except (OSError,http.client.HTTPException):
            encoded=b'';incomplete_body=True
            self.check('response-'+str(self.ordinal+1)+'-body',None,'Transport incomplete, not an empty target body')
        after=utc()
        reply={'status':response.status,'url':response_url,'headers':response_headers,'body_hex':encoded.hex(),'before':before,'after':after,'header_contract':header_result}
        if incomplete_body:reply['incomplete_body']=True
        try:reply['json']=load_json_members(encoded)
        except (ValueError,UnicodeError):reply['json']=None
        if not incomplete_body and method=='GET' and response.status==200:
            if path in ('/about','/activities','/agents') or path=='/statements' and (query or {}).get('attachments') not in ('true',True):
                self.check('response-'+str(self.ordinal+1)+'-json-object',isinstance(reply['json'],dict),
                    'Successful JSON resource response must be an object; malformed complete bodies are target violations')
        self.ordinal+=1
        evidence={'ordinal':self.ordinal,'procedure':self.current,'request':{'method':method,'path':path,'query':query or {},'continuation':continuation,'headers':{k:('[private]' if k.lower()=='authorization' else v) for k,v in request_headers.items()},'body_hex':(data or b'').hex()},'response':reply}
        with (self.out/'http.jsonl').open('a') as f:f.write(compact(evidence)+'\n')
        connection.close()
        return Reply(reply)

    def control(self, **kwargs):
        if not self.fixture.get('clock_control'):
            raise ValueError('independent clock fixture not declared')
        self.http('POST','/_control',body=kwargs, expected_statuses=None)

    def statement(self, **extra):
        return {'id':str(uuid.uuid4()),'actor':copy.deepcopy(self.actor),'verb':{'id':self.prefix+'/'+str(uuid.uuid4())},'object':{'id':self.prefix+'/'+str(uuid.uuid4())},**extra}

    def store(self, body, headers=None):
        result=self.http('PUT','/statements',{'statementId':body['id']},body,headers, expected_statuses=(204,))
        self.check('setup-write-'+body['id'],result['status']==204,'valid fixture Statement must store')
        identifier=canonical_statement_id(body['id'])
        if result['status']==204 and identifier not in self.known_statement_ids:
            if body.get('verb',{}).get('id')=='http://adlnet.gov/expapi/verbs/voided':
                self.voiding_statement_ids.add(identifier)
                target=body.get('object',{})
                if target.get('objectType')=='StatementRef' and isinstance(target.get('id'),str):
                    target_id=canonical_statement_id(target['id'])
                    if target_id not in self.voiding_statement_ids:
                        self.visible_statements.pop(target_id,None)
                        self.voided_statement_ids.add(target_id)
            self.known_statement_ids.add(identifier)
        return result

    def read(self, body, **query):
        return self.http('GET','/statements',{'statementId':body['id'],**query}, expected_statuses=(200,404))

    def statement_headers(self, reply, label):
        """Keep mandatory header failures on every observation, including pending 404."""
        if 'header_contract' in reply:
            return reply['header_contract']
        version,consistent=statement_response_headers(reply['headers'])
        self.check(label+'-version-header',version)
        self.check(label+'-consistent-through-header',consistent)
        return all_decisions([version,consistent])

    def read_visible(self, body, admission, **query):
        reply=self._read_visible(body,admission,**query)
        if reply is None:
            self.check('visibility-'+body['id']+'-dependent-checks',None,
                'Positive read precondition not established; dependent observations were not executed. Earlier FAILs remain.')
        return reply

    def _read_visible(self, body, admission, **query):
        """Observe an admitted positive Statement read; only an initial 404 may wait.

        The observation window is a harness limit, never a source deadline.
        Previously observed stored instants constrain disappearance only when
        Consistent-Through is strictly later. No previously visible 404, 200
        assertion failure or mandatory error-header failure is retried.
        Negative reads (voided or rejected) must use read(), without this helper.
        """
        label='visibility-'+body['id']+'-'+query.get('format','exact')
        identifier=canonical_statement_id(body['id'])
        if identifier in self.voided_statement_ids:
            self.check(label,None,'Known voided target requires an explicit negative read(), not a positive visibility wait')
            return None
        if admission['status'] not in [200,204]:
            self.check(label,None,'Positive observation lacks successful admission; setup failures are retained')
            return None
        window=self.fixture.get('statement_visibility_timeout_ms',5000)
        if type(window) is not int or not 0 <= window <= 60000:
            raise ValueError('statement_visibility_timeout_ms must be an integer from 0 to 60000 (harness bound only)')
        deadline=time.monotonic()+window/1000
        while True:
            reply=self.read(body,**query)
            if self.statement_headers(reply,label) is False:return None
            if reply['status']==404 and identifier in self.visible_statements:
                stored=self.visible_statements[identifier]
                headers={name.lower():value for name,value in reply['headers'].items()}
                watermark=headers.get('x-experience-api-consistent-through')
                try:
                    through,recorded=timestamp_pair(watermark,stored)
                    guaranteed=through is not None and recorded is not None and through>recorded
                    self.check(label,False if guaranteed else None,
                        'Previously visible Statement disappeared after its stored time was covered by Consistent-Through'
                        if guaranteed else 'Previously visible Statement is absent but a strictly later consistency watermark is not established; no retry or PASS',
                        {'stored':stored,'consistent_through':watermark})
                except (TypeError,ValueError,ArithmeticError):
                    self.check(label,False,'Invalid timestamp in the observed consistency context')
                return None
            if reply['status']==404:
                if time.monotonic()>=deadline:
                    self.check(label,None,'Admitted Statement not yet observable within harness window; dependent semantic checks not run. No normative deadline.',{'window_ms':window})
                    return None
                time.sleep(min(0.05,max(0,deadline-time.monotonic())))
                continue
            valid=reply['status']==200 and isinstance(reply['json'],dict) and canonical_statement_id(reply['json'].get('id'))==identifier
            self.check(label,valid,'Positive response must identify the admitted Statement')
            if not valid:return None
            stored=reply['json'].get('stored')
            try:
                timestamp_ns(stored)
                stored_valid=True
            except PrimitiveUnavailable:
                stored_valid=None
            except (TypeError,ValueError,ArithmeticError):
                stored_valid=False
            self.check(label+'-stored',stored_valid,'Retain the returned stored timestamp as the consistency witness')
            if stored_valid is False:return None
            self.visible_statements[identifier]=stored
            self.known_statement_ids.add(identifier)
            verb=reply['json'].get('verb')
            if isinstance(verb,dict) and verb.get('id')=='http://adlnet.gov/expapi/verbs/voided':
                self.voiding_statement_ids.add(identifier)
            return reply

    def conflict(self):
        known={'Check current state and use If-Match with the current ETag.':True,'document already exists':False,'Conflict':False}
        for family,path,scope,key in self.docs:
            q={**scope,key:'conflict'}
            self.http('PUT',path,q,{'v':1}, expected_statuses=(204,))
            response=self.http('PUT',path,q,{'v':2}, expected_statuses=(409,))
            self.check(family+'-409',response['status']==409,'unconditional existing document PUT')
            value=response['json']
            message=value.get('message',value.get('error')) if isinstance(value,dict) else bytes.fromhex(response['body_hex']).decode(errors='replace')
            self.check(family+'-explanation',known.get(message),'Semantic assessment of exact English messages: full message directs checking current state and retrying with current ETag in If-Match; generic conflict/existence does neither. Other text requires semantic review.',{'language':response['headers'].get('content-language','und'),'message':message})

    def consistency(self):
        if not self.fixture.get('clock_control'):
            self.check('delayed-visibility-safety',None,'Target has no declared clock/visibility control. A fixed old watermark alone does not violate a numerical recency requirement. No numerical bound is specified.')
            return
        self.control(clock='1969-12-31T23:59:59Z',hide_new=True)
        body=self.statement();self.store(body)
        self.control(clock='1970-01-01T00:00:01Z')
        for label,method,path,query in [('success','GET','/statements',{'verb':body['verb']['id']}),('error','GET','/statements',{'statementId':body['id'],'voidedStatementId':body['id']}),('head','HEAD','/statements',{'verb':body['verb']['id']})]:
            reply=self.http(method,path,query, expected_statuses=(400,) if label=='error' else (200,))
            try:ok=timestamp_ns(reply['headers']['x-experience-api-consistent-through']) <= timestamp_ns('1969-12-31T23:59:59Z')
            except PrimitiveUnavailable:ok=None
            except (KeyError,ValueError,TypeError):ok=False
            self.check('hidden-record-'+label,ok,'Watermark must not cross known stored time of invisible Statement')
        self.control(hide_new=False,release=True,clock='1970-01-01T00:00:02Z')
        reply=self.read(body)
        document=reply['json'] if reply['status']==200 else None
        self.check('released-retrieval',reply['status']==200 and isinstance(document,dict)
            and standard_uuid(document.get('id'))
            and canonical_statement_id(document['id'])==canonical_statement_id(body['id']))

    def document_ids(self, reply, label):
        """Record complete list-response defects before membership or more IO.

        Reply still raises EvidenceUnavailable for interrupted bodies, whose
        undecoded content is not evidence of a target type violation.
        """
        ids=reply['json']
        valid=reply['status']==200 and isinstance(ids,list) and all(isinstance(item,str) for item in ids)
        self.check(label+'-shape',valid,'Document ID listing must be HTTP 200 with a JSON array of strings')
        return ids if valid else None

    def documents(self):
        for family,path,scope,key in self.docs:
            q={**scope,key:'clock'}
            if self.fixture.get('clock_control'):self.control(clock='2026-10-06T00:00:01.250Z')
            created=self.http('PUT',path,q,{'v':1}, expected_statuses=(204,))
            first=self.http('GET',path,q, expected_statuses=(200,))
            if self.fixture.get('clock_control'):self.control(clock='2026-10-06T00:00:03.750Z')
            updated=self.http('PUT',path,q,{'v':2},{'If-Match':first['headers'].get('etag','*')}, expected_statuses=(204,))
            second=self.http('GET',path,q, expected_statuses=(200,))
            self.check(family+'-write-read',created['status']==updated['status']==204 and first['status']==second['status']==200)
            try:
                lm=response_http_date(second['headers']['last-modified'])
                if self.fixture.get('clock_control'):ok=timestamp_ns(lm.isoformat())==timestamp_ns('2026-10-06T00:00:03Z')
                elif self.fixture.get('host_clock_shared'):ok=timestamp_ns(updated['before']).to_integral_value(rounding='ROUND_FLOOR')<=timestamp_ns(lm.isoformat())<=timestamp_ns(updated['after']).to_integral_value(rounding='ROUND_FLOOR')
                else:ok=None
            except PrimitiveUnavailable:ok=None
            except (KeyError,ValueError,TypeError):ok=False
            self.check(family+'-last-modified',ok,'Last-Modified matches the controlled time or declared same-host UTC write interval, at HTTP-date precision')
            separated = self.fixture.get('clock_control') or (self.fixture.get('host_clock_shared') and timestamp_ns(created['after']).to_integral_value(rounding='ROUND_FLOOR') < timestamp_ns(updated['before']).to_integral_value(rounding='ROUND_FLOOR'))
            self.check(family+'-modified-moves',first['headers'].get('last-modified')!=second['headers'].get('last-modified') if separated else None,'Requires independently established write intervals in different HTTP-date seconds; a sleep is not clock evidence')
            lower='2026-10-06T00:00:03.749Z' if self.fixture.get('clock_control') else updated['before']
            upper='2026-10-06T00:00:03.751Z' if self.fixture.get('clock_control') else updated['after']
            for label,since,present in [('before',lower,True),('after',upper,False)]:
                if not (self.fixture.get('clock_control') or self.fixture.get('host_clock_shared')):
                    self.check(family+'-since-'+label,None,'No independently controlled or declared shared clock; sender timestamps are not target update bounds')
                    continue
                reply=self.http('GET',path,{**scope,'since':since}, expected_statuses=(200,))
                ids = self.document_ids(reply,family+'-since-'+label)
                valid = ids is not None
                # A shared clock only establishes an inclusive write interval.
                # At the lower bound a coarse target clock may equal before;
                # equality cannot establish the strict since-positive branch.
                exact = bool(self.fixture.get('clock_control'))
                if label == 'before' and not exact:
                    since = (host_stamp(updated['before']) - dt.timedelta(seconds=1)).isoformat().replace('+00:00','Z')
                    reply=self.http('GET',path,{**scope,'since':since}, expected_statuses=(200,))
                    ids = self.document_ids(reply,family+'-since-'+label+'-earlier')
                    valid = valid and ids is not None
                self.check(family+'-since-'+label,valid and ('clock' in ids)==present,'Independent controlled instant or declared shared-clock write bounds; since is exclusive')
            if self.fixture.get('clock_control'):
                reply=self.http('GET',path,{**scope,'since':'2026-10-06T00:00:03.750Z'}, expected_statuses=(200,))
                ids = self.document_ids(reply,family+'-since-equal')
                self.check(family+'-since-equal',ids is not None and 'clock' not in ids,'Equal timestamp is controlled independently of Last-Modified')
            else:self.check(family+'-since-equal',None,'Cannot control exact subsecond updated time; HTTP-date is insufficient')
        _,path,scope,key=self.docs[0]
        sibling={**scope,'activityId':self.prefix+'/other-scope',key:'preserved'}
        self.http('PUT',path,sibling,{'v':'sibling'}, expected_statuses=(204,))
        self.http('DELETE',path,scope, expected_statuses=(204,))
        deleted=self.http('GET',path,{**scope,key:'clock'}, expected_statuses=(404,))
        kept=self.http('GET',path,sibling, expected_statuses=(200,))
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
        if not values:
            return {}
        path.write_text(compact([{'value': value, 'ascii_only': ascii_only} for value, ascii_only in values]))
        run = subprocess.run([str(self.args.runner), '--check-identifiers', str(path)], capture_output=True, text=True, check=True)
        result = json.loads(run.stdout)
        verdicts = result.get('valid') if isinstance(result,dict) else None
        if not isinstance(verdicts,list) or len(verdicts) != len(values) or any(type(v) is not bool for v in verdicts):
            raise ValueError('Invalid grammar-check response')
        (path.with_suffix('.result.json')).write_text(run.stdout)
        return dict(zip(values, verdicts))

    def person(self):
        examples={'mbox':'mailto:unknown-'+str(uuid.uuid4())+'@example.org','mbox_sha1sum':'0123456789abcdef0123456789abcdef01234567','openid':self.prefix+'/openid','account':{'homePage':self.prefix+'/account','name':'learner'}}
        for field,value in examples.items():
            reply=self.http('GET','/agents',{'agent':compact({field:value})}, expected_statuses=(200,))
            got=reply['json']
            self.check(field+'-person',isinstance(got,dict) and got.get('objectType')=='Person',
                'The returned Person must have objectType Person')
            entries=got.get(field,[]) if isinstance(got,dict) else None
            self.check(field+'-queried',reply['status']==200 and isinstance(entries,list) and value in entries,'Unknown Agents must return the queried IFI')
            if not isinstance(got,dict):continue
            # Record every local type defect before invoking a grammar adapter.
            # An unavailable adapter must not abort later entries or IFI queries.
            typed=[]
            for key in ['name',*IFI]:
                entries=got.get(key,[])
                self.check(field+'-'+key+'-array',isinstance(entries,list))
                if not isinstance(entries,list):continue
                for index,item in enumerate(entries):
                    valid=(isinstance(item,dict) and isinstance(item.get('homePage'),str)
                           and isinstance(item.get('name'),str)) if key=='account' else isinstance(item,str)
                    self.check(field+'-'+key+'-'+str(index)+'-type',valid)
                    if valid and key!='name':typed.append((key,index,item))
            try:syntax=self.identifier_syntax(got)
            except (OSError,ValueError,subprocess.SubprocessError) as error:
                syntax={}
                self.check(field+'-identifier-grammar',None,'Grammar adapter unavailable; local defects retained',
                    {'kind':type(error).__name__,'axis':'harness-or-evidence'})
            for key,index,item in typed:
                if key=='mbox':valid=all_decisions([syntax.get((item,False)),mailbox_structure(item)])
                elif key=='mbox_sha1sum':valid=re.fullmatch(r'[0-9a-fA-F]{40}',item) is not None
                elif key=='openid':valid=syntax.get((item,True))
                else:
                    # DC-IRL needs locator evidence even when generic IRI syntax passes.
                    valid=False if syntax.get((item['homePage'],False)) is False else None
                self.check(field+'-'+key+'-'+str(index),valid,'All entries: field types plus DC-MAILTO/DC-IRL; None is an unavailable semantic adapter, never valid')
        self.check('full-external-grammar',None,'Bounded ASCII mailto and generic URI/IRI grammar only; encoded mailto and locator evidence remain concrete DC-MAILTO/DC-IRL obligations')
        self.check('person-association-inventory',None,'Partial executor checks presence and syntax only. Independently controlled no-additional-information and Agent association inventories require the manual person-semantic-identifiers procedure; random identifiers do not establish fallback applicability.')

    def metadata(self, language=False):
        if not self.fixture.get('canonical_submitted_fixture'):
            self.check('declared-inventory',None,'No declared maintained metadata inventory; submitted definitions alone cannot establish it')
            return
        definition={'name':{'en':'English','ja-JP':'日本語'},'description':{'en':'Only English'},'type':self.prefix+'/type','interactionType':'choice','choices':[{'id':'a','description':{'fr':'Français','en':'English choice'}}]}
        body=self.statement(object={'id':self.prefix+'/known-activity','definition':definition})
        if language:
            body['context']={'contextActivities':{'category':[{'id':self.prefix+'/category','definition':{'name':{'fr':'Catégorie','en':'Category'}}}]}}
        admission=self.store(body)
        if self.read_visible(body,admission) is None:return
        if not language:
            for name,activity,expected in [('known',body['object']['id'],definition),('unknown',self.prefix+'/unknown-activity',None)]:
                reply=self.http('GET','/activities',{'activityId':activity}, expected_statuses=(200,))
                got=reply['json']
                self.check(name+'-activity-id',reply['status']==200 and isinstance(got,dict) and got.get('id')==activity)
                if expected is not None:
                    self.check('available-definition',isinstance(got,dict) and got.get('definition')==expected,'Explicit fixture policy maintains all supplied properties; inventory recorded before retrieval',{'inventory':expected,'policy':self.fixture['canonical_submitted_fixture']})
            return
        maps={'/object/definition/name':definition['name'],'/object/definition/description':definition['description'],'/object/definition/choices/0/description':definition['choices'][0]['description'],'/context/contextActivities/category/0/definition/name':body['context']['contextActivities']['category'][0]['definition']['name']}
        for label,header,preferred in [('weighted','en;q=0.1, ja-JP;q=0.9','ja-JP'),('range','ja;q=1, en;q=0.5','ja-JP'),('wildcard','*',None),('unavailable','de',None),('absent',None,None)]:
            response=self.http('GET','/statements',{'statementId':body['id'],'format':'canonical'},headers={'Accept-Language':header} if header else {}, expected_statuses=(200,))
            for pointer,inventory in maps.items():
                try:
                    got=ptr(response['json'],pointer)
                    expected=preferred if preferred in inventory else 'en' if label in ['weighted','range'] and 'en' in inventory else None
                    ok=isinstance(got,dict) and len(got)==1 and all(k in inventory and inventory[k]==v for k,v in got.items()) and (expected is None or expected in got)
                except (KeyError,TypeError,IndexError):ok=False
                self.check(label+pointer,ok,'Each maintained Activity language map independently; unavailable preference permits one available language',{'inventory':inventory})

    def versions(self):
        for version in ['1.0.3','1.1.0','2.1.0']:
            reply=self.http('GET','/statements',{'limit':'1'},headers={'X-Experience-API-Version':version}, expected_statuses=(400,) if version.startswith('2.') else None)
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
                admission=self.store(body);reply=self.read_visible(body,admission)
                if reply is None:continue
                try:
                    got=ptr(reply['json'],'/object/timestamp' if nested else '/timestamp')
                    ok=timestamp_preserved(timestamp, got)
                except (KeyError,TypeError,ValueError):ok=False
                self.check(('sub-' if nested else 'top-')+timestamp,ok,'At least millisecond instant precision and UTC, permitted higher precision rounding/truncation')

    def duplicate(self):
        self.check('all-excluded-property-and-delayed-visibility-variants',None,'Finite immutable mutation only; every excluded-property duplicate and delayed fixture remains open')
        body=self.statement(object={'id':self.prefix+'/immutable','definition':{'name':{'en':'original'}}})
        admission=self.store(body)
        before=self.read_visible(body,admission)
        if before is None:
            for method in ['PUT','POST']:
                self.check(method+'-allowed-status',None,'Replay not executed: initial stored snapshot unavailable')
                self.check(method+'-all-object-preservation',None,'Comparison not executed: initial stored snapshot unavailable')
            return
        activity_before=self.http('GET','/activities',{'activityId':body['object']['id']}, expected_statuses=(200,))
        person_before=self.http('GET','/agents',{'agent':compact(body['actor'])}, expected_statuses=(200,))
        for method in ['PUT','POST']:
            changed=copy.deepcopy(body);changed['object']['definition']['name']['en']='changed'
            response=self.http(method,'/statements',{'statementId':body['id']} if method=='PUT' else {},changed, expected_statuses=(204,409) if method=='PUT' else (200,204,409))
            self.check(method+'-allowed-status',response['status'] in ([204,409] if method=='PUT' else [200,204,409]))
            after=self.read_visible(body,admission)
            if after is None:
                self.check(method+'-all-object-preservation',None,'Comparison not executed: returned stored snapshot unavailable')
                continue
            activity_after=self.http('GET','/activities',{'activityId':body['object']['id']}, expected_statuses=(200,))
            person_after=self.http('GET','/agents',{'agent':compact(body['actor'])}, expected_statuses=(200,))
            self.check(method+'-all-object-preservation',all_decisions([
                snapshot_equal(before['json'],after['json']),
                snapshot_equal(activity_before['json'],activity_after['json']),
                snapshot_equal(person_before['json'],person_after['json'])]),
                'Same stored values, including managed fields; equivalent representation and designated member order do not prove mutation')

    def voiding(self):
        a=self.statement();admitted_a=self.store(a)
        if self.read_visible(a,admitted_a) is None:return
        b=self.statement(verb={'id':'http://adlnet.gov/expapi/verbs/voided'},object={'objectType':'StatementRef','id':a['id']});admitted_b=self.store(b)
        if self.read_visible(b,admitted_b) is None:return
        c=self.statement(verb={'id':'http://adlnet.gov/expapi/verbs/voided'},object={'objectType':'StatementRef','id':b['id']})
        response=self.http('PUT','/statements',{'statementId':c['id']},c, expected_statuses=(204,400,403,409))
        self.check('c-allowed-status',response['status'] in [204,400,403,409])
        for name,body,admission,visible in [('a',a,admitted_a,False),('b',b,admitted_b,True),('c',c,response,response['status']==204)]:
            if name=='c' and response['status'] not in [204,400,403,409]:
                self.check('c-retrievability',None,'No permitted admission branch');continue
            single=self.read_visible(body,admission) if visible else self.read(body)
            if single is None:
                self.check(name+'-retrievability',None,'Positive visibility precondition not established; earlier failures retained');continue
            if not visible:self.statement_headers(single,name+'-negative')
            self.check(name+'-single-retrievability',single['status']==(200 if visible else 404))
            listing=self.collect_statements({'verb':body['verb']['id']},name+'-query',forbidden_ids={body['id']} if not visible else set())
            self.check(name+'-retrievability',None if listing is None else (canonical_statement_id(body['id']) in {canonical_statement_id(s['id']) for s in listing})==visible,
                'Use complete pagination for absent B/C; retain an observed forbidden A even if later traversal is unavailable')

    def recursive_formats(self):
        from lrs_projection_responses import format_cases
        vectors=[c for c in format_cases() if c['id'].endswith('-ids') and any(x.get('kind')=='json-shape' for s in c['steps'] for x in s['expect'].get('checks',[]))]
        for case in vectors:
            body=copy.deepcopy(case['steps'][0]['request']['body']);body['id']=str(uuid.uuid4())
            response=self.store(body)
            if response['status']!=204:continue
            for format_name in ['exact','ids','canonical']:
                reply=self.read_visible(body,response,format=format_name)
                if reply is None:continue
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
                    self.check(case['id']+'-'+format_name,False if any(o['outcome']=='failed' for o in obs) else None if any(o['outcome']=='not-run' for o in obs) else True,details=obs)
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
                reply=self.http(req['method'],req['path'],query,body, expected_statuses=(exp['status'],))
                self.check(case['id']+'-acceptance',reply['status'] in [200,204],'Required acceptance vector')
                if reply['status'] not in [200,204]:continue
                visible=self.read_visible(body,reply)
                if visible is None:continue
                returned=visible['json'];obs=preservation(body,returned)
                if isinstance(returned,dict):
                    for key in ['id','stored','timestamp','authority','version']:
                        valid=key in returned and (isinstance(returned[key],dict) if key=='authority' else isinstance(returned[key],str))
                        if key in ['stored','timestamp'] and valid:
                            try:timestamp_ns(returned[key])
                            except PrimitiveUnavailable:valid=None
                            except (ValueError,TypeError):valid=False
                        if key=='id' and valid:
                            try:uuid.UUID(returned[key])
                            except ValueError:valid=False
                        obs.append({'pointer':'/'+key,'outcome':'not-run' if valid is None else 'passed' if valid else 'failed','rule':'required returned field type; combined date/time and delegated capability boundary'})
                with (self.out/'preservation.jsonl').open('a') as f:f.write(compact({'case':case['id'],'observations':obs})+'\n')
                self.check(case['id']+'-preservation',False if any(o['outcome']=='failed' for o in obs) else None if any(o['outcome']=='not-run' for o in obs) else True,'Every submitted pointer walked; see preservation.jsonl')
                tested+=1
        self.check('coverage',True,details={'accepted_vectors_read':tested})
        self.check('omission-post-batches-two-attachment-raw-parts',None,'Omissions, all repeated POST batches, and both distinct attachment multipart fixtures remain an explicit separate rubric gap')

    def http_rfc(self):
        for family,path,scope,key in self.docs:
            q={**scope,key:'http'}
            self.http('PUT',path,q,{'text':'UTF-8 日本語'}, expected_statuses=(204,))
            get=self.http('GET',path,q, expected_statuses=(200,));head=self.http('HEAD',path,q,headers={'x-experience-api-version':'2.0.0'}, expected_statuses=(200,))
            self.check(family+'-head',get['status']==head['status']==200 and not head['body_hex'])
            self.check(family+'-head-version',head['headers'].get('x-experience-api-version')=='2.0.0')
            self.check(family+'-head-metadata-recommendation',all(get['headers'].get(k)==head['headers'].get(k) for k in ['content-type','etag','last-modified']), 'RFC2616 9.4 SHOULD; record omitted/different values without mandatory FAIL', {'get':get['headers'],'head':head['headers']})
            current_tag=get['headers'].get('etag')
            self.check(family+'-etag-present',current_tag is not None,'ETag required on this successful document GET')
            if current_tag is None:
                self.check(family+'-conditional-probe',None,'No captured ETag; dependent request not issued')
                continue
            opaque=current_tag[2:] if current_tag.startswith('W/') else current_tag
            quoted=len(opaque)>=2 and opaque.startswith('"') and opaque.endswith('"')
            self.check(family+'-etag-quoted',quoted,'Entity-tag is quoted; full grammar remains in HTTP contract')
            if not quoted:continue
            different_tag=opaque[:-1]+'!"'
            stale=self.http('PUT',path,q,{'text':'changed'},{'iF-mAtCh':different_tag}, expected_statuses=(412,))
            self.check(family+'-case-insensitive-if-match',stale['status']==412)
        self.check('complete-rfc-header-context-framing-grammar',None,'Finite HEAD and header casing probes do not discharge every HTTP RFC requirement')

    def mime(self):
        data=b'\x00binary\xff\r\n--boundary-near-but-not-delimiterXX\r\nend'
        digest=hashlib.sha256(data).hexdigest()
        for label,boundary,quoted in [('quoted','boundary-near-but-not-delimiter-outer',True),('ordinary','normal-boundary',False)]:
            body=self.statement(attachments=[{'usageType':self.prefix+'/attachment','display':{'en':'binary'},'contentType':'application/octet-stream','length':len(data),'sha2':digest}])
            raw=b'preamble\r\n--'+boundary.encode()+b'\r\nContent-Type: application/json\r\n\r\n'+compact(body).encode()+b'\r\n--'+boundary.encode()+b'\r\ncOnTeNt-TyPe: application/octet-stream\r\nX-Experience-API-Hash: '+digest.encode()+b'\r\nContent-Transfer-Encoding: binary\r\n\r\n'+data+b'\r\n--'+boundary.encode()+b'--\r\nepilogue'
            media='multipart/mixed; boundary='+('"'+boundary+'"' if quoted else boundary)
            parsed=BytesParser(policy=email.policy.default).parsebytes(('Content-Type: '+media+'\r\nMIME-Version: 1.0\r\n\r\n').encode()+raw)
            parts=list(parsed.iter_parts())
            self.check(label+'-independent-input-parser',len(parts)==2 and parts[1].get_payload(decode=True)==data)
            response=self.http('PUT','/statements',{'statementId':body['id']},headers={'Content-Type':media},raw=raw, expected_statuses=(204,))
            self.check(label+'-accepted',response['status']==204)
            if response['status']==204:
                if self.read_visible(body,response) is None:continue
                returned=self.http('GET','/statements',{'statementId':body['id'],'attachments':'true'}, expected_statuses=(200,))
                message=BytesParser(policy=email.policy.default).parsebytes(('Content-Type: '+returned['headers'].get('content-type','')+'\r\nMIME-Version: 1.0\r\n\r\n').encode()+bytes.fromhex(returned['body_hex']))
                output=list(message.iter_parts()) if message.is_multipart() else []
                decoded=[part.get_payload(decode=True) for part in output] if len(output)==2 and all(not part.is_multipart() for part in output) else []
                valid=len(decoded)==2 and all(isinstance(payload,bytes) for payload in decoded)
                self.check(label+'-returned-part-structure',valid,'Expected two leaf MIME parts with decodable byte payloads')
                self.check(label+'-returned-binary-hash',valid and hashlib.sha256(decoded[1]).hexdigest()==digest and decoded[1]==data)
        self.check('legal-folding-transfer-variations-full-rfc',None,'These two witnesses do not close all MIME framing/transport classes')

    def limits(self):
        if self.fixture.get('size_limit'):
            response=self.http('POST','/statements',body=self.statement(result={'response':'x'*(self.fixture['size_limit']+1)}), expected_statuses=(413,))
            self.check('declared-size-overrun',response['status']==413)
        else:self.check('declared-size-overrun',None,'No numerical limit fixture declared')
        if self.fixture.get('rate_path'):
            time.sleep(self.fixture['rate_window_seconds']+.1)
            trigger=self.fixture['rate_trigger_request_count']
            statuses=[self.http('GET',self.fixture['rate_path'], expected_statuses=(429,) if index+1==trigger else (200,))['status'] for index in range(trigger)]
            self.check('declared-rate-overrun',statuses==[200]*(trigger-1)+[429],'Declared trigger includes the denied request; begin after one idle window',details=statuses)
        else:self.check('declared-rate-overrun',None,'No rate fixture declared')
        for path in ['/statements','/about','/agents','/activities',*[d[1] for d in self.docs]]:
            parameter = 'unknownProcedureParameter'
            declarations = self.fixture.get('parameter_vocabulary', [])
            declared = any(c.get('method') == 'GET' and c.get('path') == path and c.get('evidence', '').strip() and parameter in c.get('unrecognized', []) for c in declarations)
            if not declared:
                self.check(path+'-unknown',None,'Requires independent method/path vocabulary declaring this parameter unrecognized; names and observed 400 are not evidence')
                continue
            query = {'agent':compact(self.actor)} if path == '/agents' else {'activityId':self.prefix} if path == '/activities' else next((dict(scope) for _,p,scope,_ in self.docs if p == path), {})
            self.check(path+'-unknown',self.http('GET',path,{**query,parameter:'value'}, expected_statuses=(400,))['status']==400)
        self.check('all-size-doc-attachment-extension-contexts',None,'Only declared Statement body and rate limits exercised; no invented document or attachment limit')

    def authorization(self):
        policy=self.fixture.get('authorization_policy')
        self.check('decision-boundary',True if policy else None,'Fixture information from configured policy, not a normative authority',policy)
        invalid=os.environ.get('KIVA_CONFORMANCE_INVALID_TOKEN');reader=os.environ.get('KIVA_CONFORMANCE_READER_TOKEN');writer=os.environ.get('KIVA_CONFORMANCE_WRITER_A_TOKEN')
        if not all([invalid,reader,writer]):self.check('credential-fixtures',None,'Refused/permitted/operation-denied credential fixtures unavailable');return
        response=self.http('GET','/about',headers={'Authorization':'Bearer '+invalid}, expected_statuses=(401,))
        self.check('refused-401',response['status']==401)
        denied_body=self.statement()
        response=self.http('PUT','/statements',{'statementId':denied_body['id']},denied_body,{'Authorization':'Bearer '+reader}, expected_statuses=(403,))
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
                admission=self.store(body,{'Authorization':'Bearer '+token} if token else {})
                reply=self.read_visible(body,admission)
                if reply is None:continue
                value=reply['json'].get('authority') if isinstance(reply['json'],dict) else None
                self.check(name+'-authority-present-'+body['id'],agent_validity(value),'Agent/Group IFI types and syntax, including every member; unsupported mailto is unavailable; this check does not discharge separate DC-IRL locator semantics')
                current.append(value)
            self.check(name+'-deterministic',agent_identity_equal(current[0],current[1]) if len(current)==2 else None,'Both admitted Statements observable; bounded mapping comparison permits unordered members; changing display facts require full declared mapping contract')
            authorities.append(current)
        self.check('distinct-credential-identities',None,'Different credentials need not produce different authorities; injectivity is not a normative requirement',details=authorities)

    def related_authority(self):
        run=subprocess.run([os.sys.executable,str(ROOT/'scripts/ci/probe_lrs_authority_filter.py'),'--endpoint',self.args.endpoint,'--runner',str(self.args.runner),'--output',str(self.out/'authority-filter')],capture_output=True,text=True)
        (self.out/'authority-filter.log').write_text(run.stdout+run.stderr)
        try:
            outcome=json.loads((self.out/'authority-filter'/'outcome.json').read_text())
        except (OSError, ValueError):outcome={}
        if not isinstance(outcome,dict):outcome={}
        status=outcome.get('status')
        verdict=False if status=='failed' else True if status=='passed' and run.returncode==0 else None
        self.check('actual-returned-authority-true-default-false',verdict,'Explicit helper result; unavailable fixture/child incomplete is never a target failure or pass',{'exit_code':run.returncode,'helper_outcome':outcome})
        checks=outcome.get('checks')
        valid=isinstance(checks,list) and bool(checks) and all(isinstance(c,dict) for c in checks)
        identified=False
        if valid:
            ids=[c.get('id') for c in checks]
            identified=(all(isinstance(identity,str) and identity for identity in ids)
                        and len(set(ids))==len(ids))
            for check in checks:
                observed={'passed':True,'failed':False}.get(check.get('outcome'))
                identity=check.get('id') if isinstance(check.get('id'),str) and check['id'] else 'unidentified'
                self.check('authority-helper-'+identity,observed,check.get('reason','Child response observation'),check)
        complete=(valid and identified and outcome.get('execution_complete') is True
                  and all(c.get('outcome') in ('passed','failed') for c in checks))
        self.check('authority-required-checks-completed',True if complete else None,
                   'Every applicable query must be observed; a bootstrap or child FAIL cannot complete skipped queries')

    def languages(self):
        # Fixed RFC5646/IANA definitions; validity is not an optional parser
        # preference. All other Statement fields and admission conditions hold.
        for tag,valid in [('en',True),('zh-Hant-CN',True),('i-klingon',True),('x-private',True),('abcdefgh',False),('abcdefghi',False),('en-abcdefghi',False),('sl-rozaj-rozaj',False),('en-a-foo-a-bar',False)]:
            for nested in [False,True]:
                body=self.statement(object={'id':self.prefix+'/lang/'+str(uuid.uuid4()),'definition':{'name':{tag:'plain text <b>'}}})
                if nested:
                    sub=copy.deepcopy(body);sub.pop('id');body=self.statement(object={'objectType':'SubStatement',**sub})
                response=self.http('PUT','/statements',{'statementId':body['id']},body, expected_statuses=(204,) if valid else (400,))
                self.check(tag+('-sub' if nested else '-top'),response['status']==(204 if valid else 400),'DC-LANGUAGE: fixed vectors characterized against RFC5646 and the pinned IANA registry; the registry is reference evidence, not loaded by this executor; full registry-based validation remains unexecuted')
        self.check('full-rfc-every-language-map-location',None,'Defined by issued-definition-contracts.json DC-LANGUAGE; this partial executor covers Activity.definition.name at root and SubStatement only')

    def references(self):
        a=self.statement();admitted_a=self.store(a)
        b=self.statement(object={'objectType':'StatementRef','id':a['id']});admitted_b=self.store(b)
        c=self.statement(object={'objectType':'StatementRef','id':b['id']});admitted_c=self.store(c)
        ready=[self.read_visible(body,admission) for body,admission in [(a,admitted_a),(b,admitted_b),(c,admitted_c)]]
        if any(r is None for r in ready):return
        for field,value in [('verb',a['verb']['id']),('activity',a['object']['id']),('agent',compact(a['actor']))]:
            rows=self.collect_statements({field:value},field)
            self.check(field+'-recursive-inheritance',None if rows is None else {canonical_statement_id(v['id']) for v in (a,b,c)}<={canonical_statement_id(v['id']) for v in rows})
        context=self.statement(context={'statement':{'objectType':'StatementRef','id':a['id']}});admission=self.store(context)
        if self.read_visible(context,admission) is None:return
        rows=self.collect_statements({'verb':a['verb']['id']},'context',forbidden_ids={context['id']})
        self.check('context-does-not-inherit',None if rows is None else canonical_statement_id(context['id']) not in {canonical_statement_id(v['id']) for v in rows})
        self.check('controlled-time-registration-limit-cycles-voided-targets',None,'Remaining reference combinations and cycle fixtures not yet controlled')

    def page_last_modified(self, headers, items):
        """Page-local maximum; None items means the body is unavailable.

        A known stored time above the header proves a maximum violation even
        if another time needs a provider. Missing/invalid values also dominate
        provider uncertainty. A supplied header always needs valid syntax.
        """
        header_time=None
        if 'last-modified' in headers:
            try:
                parsed=response_http_date(headers['last-modified'])
                header_time=timestamp_ns(parsed.isoformat()).to_integral_value(rounding='ROUND_FLOOR')
            except PrimitiveUnavailable:
                pass
            except (ValueError,TypeError,ArithmeticError):
                return False
        elif items is not None and items:
            return False
        if items is None:
            return None
        if not items:
            return True if 'last-modified' not in headers or header_time is not None else None
        unavailable=header_time is None
        known=[]
        for item in items:
            try:
                known.append(timestamp_ns(item['stored']).to_integral_value(rounding='ROUND_FLOOR'))
            except PrimitiveUnavailable:
                unavailable=True
            except (KeyError,ValueError,TypeError,ArithmeticError):
                return False
        if header_time is not None and any(value>header_time for value in known):
            return False
        if unavailable:
            return None
        return max(known)==header_time

    def collect_statements(self, query, label, forbidden_ids=frozenset()):
        """Retain known exclusions per page; absence needs a terminal chain."""
        rows=[];more=None;seen=set();base=None;complete=True
        forbidden_ids={canonical_statement_id(value) for value in forbidden_ids}
        for page in range(100):
            try:
                reply=self.http('GET','/statements',query if more is None else None,continuation=more,
                                continuation_base=base,retain_partial=True, expected_statuses=(200,))
            except (OSError,ValueError,http.client.HTTPException):
                self.check(label+'-pages',None,'Continuation transport/routing unavailable; no incomplete-set membership verdict')
                return None
            self.statement_headers(reply,label+'-page-'+str(page))
            if reply['status']!=200:
                self.check(label+'-page-status',False,'StatementResult page must return 200')
                return None
            if reply.get('incomplete_body'):
                self.check(label+'-page-last-modified',self.page_last_modified(reply['headers'],None),
                    'Received header syntax is independent of unavailable page maximum')
                self.check(label+'-page-body',None,'Body transport incomplete; observed header failures retained')
                return None
            value=reply['json']
            if not isinstance(value,dict) or not isinstance(value.get('statements'),list):
                self.check(label+'-page-shape',False,'StatementResult must contain a statements array')
                return None
            items=value['statements']
            valid_items=[item for item in items if isinstance(item,dict) and isinstance(item.get('id'),str)]
            valid=len(valid_items)==len(items)
            self.check(label+'-page-shape',valid,'Every returned Statement must be an object with a string id')
            complete=complete and valid
            rows.extend(valid_items)
            self.check(label+'-observed-exclusions-'+str(page),not any(canonical_statement_id(item['id']) in forbidden_ids for item in valid_items),
                'A forbidden object already observed cannot be removed by later pages or transport failure')
            self.check(label+'-page-last-modified',self.page_last_modified(reply['headers'],items),
                'Page header compared with this page; known defects retained before provider delegation')
            more=value.get('more','')
            if not isinstance(more,str):
                self.check(label+'-more-type',False);return None
            if more=='':return rows if complete else None
            if not iri_reference_valid(more):
                self.check(label+'-more-syntax',False,'Malformed continuation IRI reference');return None
            base=reply.get('url')
            try:destination=urljoin(base or '',more)
            except ValueError:
                self.check(label+'-pages',None,'Valid continuation needs unsupported resolution capability');return None
            if destination in seen:
                self.check(label+'-pages',None,'Repeated continuation exceeds bounded collector capability');return None
            seen.add(destination)
        self.check(label+'-pages',None,'Harness page bound reached; no normative size limit or absence verdict')
        return None

    def interaction(self):
        policy=self.fixture.get('interaction_policy')
        self.check('declared-policy',True if policy else None,details=policy)
        for kind,statuses in [('invalid-type',[400]),('choice',[204,400]),('performance',[204,400])]:
            definition={'interactionType':kind,'correctResponsesPattern':['{case_matters=true}arbitrary']}
            if kind=='choice':definition['choices']=[{'id':'a','description':{'en':'A'}}]
            if kind=='performance':definition['steps']=[{'id':'a','description':{'en':'A'}}]
            body=self.statement(object={'id':self.prefix+'/'+kind,'definition':definition})
            response=self.http('PUT','/statements',{'statementId':body['id']},body, expected_statuses=tuple(statuses))
            self.check(kind+'-strength',response['status'] in statuses)
        self.check('all-components-patterns-characterstring-parameters',None,'Optional acceptance is permitted; exhaustive chosen validation policy not yet reconciled')

    def advisory_metadata(self):
        self.check('declared-network-metadata-policy',reason='Policy observation only; no mandatory network fetch',details=self.fixture.get('resolvable_metadata_policy','undeclared'))
        self.check('controlled-hosted-metadata-observation',None,'No controlled external metadata host supplied; optional behavior remains unobserved')

    def rejection(self):
        body=self.statement(timestamp=utc())
        raw=compact(dict(reversed(list(body.items())))).encode()
        reply=self.http('PUT','/statements',{'statementId':body['id']},raw=raw, expected_statuses=(204,))
        self.check('reordered-json',reply['status']==204)
        missing=self.statement(verb={'id':'http://adlnet.gov/expapi/verbs/voided'},object={'objectType':'StatementRef','id':str(uuid.uuid4())})
        self.store(missing)
        self.check('future-margin-and-batch-advisory',None,'No declared future clock margin or complete whole-batch policy witness; no invented numeric margin')

    def cursor(self):
        self.check('metadata-and-pagination-policy',reason='Allowed implementation choice, not a mandatory alternative',details=self.fixture.get('cursor_policy','undeclared'))
        a=self.statement();self.store(a)
        self.http('GET','/statements',{'limit':'1'}, expected_statuses=(200,))
        b=self.statement();self.store(b)
        self.check('cursor-mutation-observation',None,'Initial page recorded; concurrent cursor replay/voiding and authorized metadata choices remain unobserved')

    def iri(self):
        policy=self.fixture.get('iri_strategy')
        self.check('declared-allowed-strategy',True if policy in ['simple','syntax-normalization','simple-and-syntax-normalization'] else None,'RFC3987 allowed strategy declaration',policy)
        for suffix in ['路径','%70ath','Path','path','a/../path']:
            body=self.statement(object={'id':self.prefix+'/'+suffix})
            admission=self.store(body);reply=self.read_visible(body,admission)
            if reply is None:continue
            document=reply['json']
            activity=document.get('object') if isinstance(document,dict) else None
            self.check(suffix+'-received-identity',isinstance(activity,dict)
                and isinstance(activity.get('id'),str) and activity['id']==body['object']['id'],
                'Returned Activity must be an object with the submitted string identifier')
        self.check('normalization-comparison-and-default-port',None,'Accepted IRI preservation does not establish equality strategy; external grammar/equality classes still open')

    def execute(self):
        catalog=load_procedures(ROOT/'spec/conformance/lrs-review-tests.yaml')
        functions=[self.conflict,self.consistency,self.http_rfc,self.mime,self.limits,self.duplicate,self.recursive_formats,self.references,lambda:self.metadata(True),self.documents,self.person,self.metadata,self.versions,self.authorization,self.interaction,self.advisory_metadata,self.rejection,self.authority,self.cursor,self.iri,self.languages,self.precision,self.tables,self.related_authority,self.voiding]
        for spec,function in zip(catalog,functions,strict=True):
            name=spec['id']
            if self.args.procedure and name not in self.args.procedure:continue
            self.current=name
            self.current_spec=spec
            self.results[name]={'level':spec['level'],'checks':[],'rubric_sha256':hashlib.sha256(compact(spec).encode()).hexdigest()}
            try:function()
            except Exception as error:self.check('execution-unavailable',None,str(error),
                {'kind':type(error).__name__,'axis':'harness-or-evidence','target_violation':False})
            outcomes={c['outcome'] for c in self.results[name]['checks']}
            self.results[name]['outcome']='failed' if 'failed' in outcomes else 'incomplete' if 'not-run' in outcomes else 'observed' if spec['level']=='advisory' else 'passed'
            print(name,self.results[name]['outcome'],flush=True)
        summary={'schema_version':1,'axis':self.fixture['axis'],'scope':'Executed finite rubric subconditions; unexecuted subconditions retained. No whole-standard closure.','target':self.args.endpoint,'fixture':self.fixture,'input_sha256':{str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p):sha(p) for p in [Path(__file__),ROOT/'scripts/ci/lrs_preservation.py',self.args.fixture,ROOT/'spec/conformance/lrs-review-tests.yaml',self.args.runner]},'procedures':self.results}
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
