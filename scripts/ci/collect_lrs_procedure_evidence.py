#!/usr/bin/env python3
"""Assemble explicitly reviewed rubric-to-execution links, retaining every gap.

This is an author discharge map, not independent acceptance. Related rule IDs,
counts, and successful neighboring tests do not establish semantic coverage.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path

from lrs_preservation import dump_json_members, statement_response_headers
from lrs_procedure_definitions import load_procedures
from report_issued_test_results import (ROOT, definition_obligations, procedure_incomplete,
    procedure_coverage_complete, file_sha256, unresolved_http_checks, implementation_inputs,
    verify_passing_http)
from run_lrs_native_controls import verify as verify_native

FILES = ('lrs-review-tests.yaml','issued-test-procedures.yaml',
         'issued-supplemental-tests.yaml','issued-definition-tests.yaml')


def rubric_hash(spec):
    return hashlib.sha256(dump_json_members(spec).encode()).hexdigest()


def expand_obligations(selectors, obligations):
    if not isinstance(selectors, list) or not selectors:
        raise ValueError('empty rubric selection')
    selected = set()
    for selector in selectors:
        matches = {p for p in obligations if p == selector or p.startswith(selector+'/')}
        if not matches or selected & matches:
            raise ValueError('empty or overlapping rubric selector: '+selector)
        selected.update(matches)
    return selected


def http_observation(case):
    outcome = case.get('outcome','not-run')
    if outcome == 'failed' or case.get('failures'):
        return 'failed'
    return 'passed' if outcome == 'passed' and not unresolved_http_checks(case) else 'not-run'


def native_http(directory):
    by_test = {}
    for path in sorted((directory/'http').glob('*.json')):
        record=json.loads(path.read_text())
        identifier='kiva-server::'+record['test']
        received=record.get('received_response')
        if not received or 'body_hex' not in received:
            by_test.setdefault(identifier,[]).append({'artifact':str(path.relative_to(directory)),
                'sha256':file_sha256(path),'outcome':'not-run'})
            continue
        headers={}
        duplicate=False
        for header in received['headers']:
            name=header['name'].lower()
            if name in headers and name in ('x-experience-api-version','x-experience-api-consistent-through'):
                duplicate=True
            headers[name]=bytes.fromhex(header['value_hex']).decode('latin1')
        valid=(headers.get('x-experience-api-version')=='2.0.0' and not duplicate
               and received['status']==record['response']['status'])
        if record['request']['uri'].split('?',1)[0].startswith('/statements'):
            version, consistent=statement_response_headers(headers)
            valid=valid and version and consistent
        if record['request']['method']=='HEAD':
            valid=valid and received['body_hex']==''
        by_test.setdefault(identifier,[]).append({'artifact':str(path.relative_to(directory)),
            'sha256':file_sha256(path),'outcome':'passed' if valid is True else 'not-run' if valid is None else 'failed'})
    return by_test


def resolve_witness(witness, native, traces, http, partial, backend, completed=None):
    kind=witness['kind']
    identity=witness['id'].replace('{backend}',backend)
    if kind=='procedure':
        result=(completed or {}).get(identity)
        if result is None:
            return {'kind':kind,'id':identity,'outcome':'not-run','reason':'missing evaluated branch'}
        outcome=('failed' if result['outcome']=='failed' else 'passed'
                 if result['execution_complete'] and result['coverage_complete'] else 'not-run')
        return {'kind':kind,'id':identity,'outcome':outcome,'rubric_sha256':result['rubric_sha256'],
                'checks':[check['id'] for check in result['checks']]}
    if kind=='native':
        result=native['tests'].get(identity)
        if result is None:
            return {'kind':kind,'id':identity,'outcome':'not-run','reason':'missing executed native control'}
        observation={'kind':kind,'id':identity,**result}
        if witness.get('http'):
            observation['http']=traces.get(identity,[])
            outcomes={row['outcome'] for row in observation['http']} | {result['outcome']}
            if not observation['http']:
                outcomes.add('not-run')
            observation['outcome']='failed' if 'failed' in outcomes else 'passed' if outcomes=={'passed'} else 'not-run'
        return observation
    if kind=='http':
        case=http.get(identity)
        return {'kind':kind,'id':identity,'outcome':http_observation(case) if case else 'not-run'}
    if kind=='partial':
        matches=[c for c in partial if c['id']==identity or witness.get('prefix') and c['id'].startswith(identity)]
        outcomes={c['outcome'] for c in matches}
        return {'kind':kind,'id':identity,'checks':[c['id'] for c in matches],
            'outcome':'failed' if 'failed' in outcomes else 'passed' if matches and outcomes <= {'passed','observed'} else 'not-run'}
    raise ValueError('unknown witness kind: '+kind)


def dependency_order(catalog, manifest):
    """Only explicit curated branch links compose; cycles cannot prove themselves."""
    visiting=set()
    finished=set()
    ordered=[]
    def visit(name):
        if name not in catalog:
            raise ValueError('unknown procedure dependency: '+name)
        if name in visiting:
            raise ValueError('circular procedure dependency: '+name)
        if name in finished:
            return
        visiting.add(name)
        for binding in manifest.get('procedures',{}).get(name,{}).get('bindings',[]):
            for witness in binding.get('witnesses',[]):
                if witness['kind']=='procedure':
                    visit(witness['id'])
        visiting.remove(name)
        finished.add(name)
        ordered.append(name)
    for name in catalog:
        visit(name)
    return ordered


def assemble(catalog, manifest, partial_report, native, traces, http, backend):
    if manifest.get('schema_version')!=1 or set(manifest.get('procedures',{}))-set(catalog):
        raise ValueError('invalid procedure discharge inventory')
    output={}
    for name in dependency_order(catalog,manifest):
        spec=catalog[name]
        obligations=definition_obligations(spec)
        previous=partial_report.get('procedures',{}).get(name)
        if previous and previous.get('rubric_sha256')!=rubric_hash(spec):
            raise ValueError('stale partial procedure: '+name)
        partial=copy.deepcopy(previous['checks']) if previous else []
        if len({c['id'] for c in partial}) != len(partial):
            raise ValueError('duplicate partial observation: '+name)
        result={'level':spec['level'],'rubric_sha256':rubric_hash(spec),'checks':copy.deepcopy(partial),'definition_coverage':{}}
        entry=manifest['procedures'].get(name)
        if entry:
            if entry.get('rubric_sha256')!=rubric_hash(spec):
                raise ValueError('stale semantic discharge map: '+name)
            seen=set()
            for index,binding in enumerate(entry['bindings']):
                pointers=expand_obligations(binding['obligations'],obligations)
                if pointers & seen or not binding.get('rationale','').strip() or not binding.get('witnesses'):
                    raise ValueError('invalid or overlapping semantic binding: '+name)
                seen |= pointers
                witnesses=[resolve_witness(w,native,traces,http,partial,backend,output) for w in binding['witnesses']]
                outcomes={w['outcome'] for w in witnesses}
                outcome='failed' if 'failed' in outcomes else 'passed' if outcomes=={'passed'} else 'not-run'
                identifier='discharge-'+str(index)
                check={'id':identifier,'outcome':outcome,'reason':binding['rationale'],'details':{'witnesses':witnesses}}
                result['checks'].append(check)
                for pointer in pointers:
                    result['definition_coverage'][pointer]={'sha256':obligations[pointer],'checks':[identifier]}
                # Only explicitly named, exact historical capability gaps can be
                # replaced by completed witnesses. Never replace an observed FAIL
                # or a newly unavailable runtime/transport check.
                for replacement in binding.get('supersedes',[]):
                    matches=[c for c in partial if c['id']==replacement['id']]
                    if len(matches)!=1 or matches[0].get('reason')!=replacement['reason']:
                        raise ValueError('supersession is not the declared capability gap: '+name)
                    old=matches[0]
                    if old['outcome']=='not-run' and outcome=='passed':
                        result['checks'].remove(old)
                        check['details'].setdefault('superseded_capability_gaps',[]).append(old)
        missing=sorted(set(obligations)-set(result['definition_coverage']))
        if missing:
            result['checks'].append({'id':'uncovered-definition-obligations','outcome':'not-run',
                'reason':'No reviewed execution witness for these definition leaves','details':missing})
        outcomes={c['outcome'] for c in result['checks']}
        result['outcome']='failed' if 'failed' in outcomes else 'incomplete' if 'not-run' in outcomes else 'passed'
        result['coverage_complete']=procedure_coverage_complete(result,spec)
        result['execution_complete']=not procedure_incomplete(result,spec)
        output[name]=result
    return {'schema_version':1,'scope':'explicit author semantic discharge; no independent acceptance',
        'backend':backend,'procedures':output}


def collect_capture(directory, backend, root=ROOT):
    """Reconstruct from original artifacts; never accept a standalone PASS label."""
    native_dir=directory/'native'
    native=verify_native(native_dir,backend,inputs=implementation_inputs(root))
    http=json.loads((directory/'http.json').read_text())
    cases={case['id']:case for suite in http['suites'] for case in suite['cases']}
    if len(cases)!=sum(len(s['cases']) for s in http['suites']):
        raise ValueError('duplicate HTTP evidence')
    catalog={spec['id']:spec for file in FILES for spec in load_procedures(root/'spec/conformance'/file)}
    manifest=root/'spec/conformance/implementation-procedure-bindings.json'
    partial=directory/'procedures/summary.json'
    result=assemble(catalog,json.loads(manifest.read_text()),json.loads(partial.read_text()),
                    native,native_http(native_dir),cases,backend)
    result['input_sha256']={
        'http.json':file_sha256(directory/'http.json'),
        'native/receipt.json':file_sha256(native_dir/'receipt.json'),
        'procedures/summary.json':file_sha256(partial),
        'spec/conformance/implementation-procedure-bindings.json':file_sha256(manifest),
    }
    return result


def combined_wrapper_exit(partial_exit, assembled, http, root=ROOT):
    """Only resolved capability gaps may turn the partial executor's 2 into 0."""
    if partial_exit not in (0,2):
        return partial_exit
    procedures=assembled.get('procedures',{})
    if any(p['outcome']=='failed' for p in procedures.values()):
        return 1
    if not procedures or not all(p['execution_complete'] and p['coverage_complete'] for p in procedures.values()):
        return 2
    try:
        verify_passing_http(http,root)
    except (ValueError,KeyError,TypeError):
        return partial_exit or 2
    return 0


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('http','native','partial','manifest','output'):
        parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--backend',choices=('memory','postgres'),required=True)
    args=parser.parse_args()
    native=verify_native(args.native,args.backend)
    http=json.loads(args.http.read_text())
    cases={case['id']:case for suite in http['suites'] for case in suite['cases']}
    if len(cases)!=sum(len(s['cases']) for s in http['suites']):
        raise ValueError('duplicate HTTP evidence')
    catalog={spec['id']:spec for file in FILES for spec in load_procedures(ROOT/'spec/conformance'/file)}
    result=assemble(catalog,json.loads(args.manifest.read_text()),json.loads(args.partial.read_text()),
                    native,native_http(args.native),cases,args.backend)
    result['input_sha256']={str(p):file_sha256(p) for p in
                           [args.http,args.native/'receipt.json',args.partial,args.manifest]}
    args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
    complete=all(p['execution_complete'] and p['coverage_complete'] for p in result['procedures'].values())
    failed=any(p['outcome']=='failed' for p in result['procedures'].values())
    return 1 if failed else 0 if complete else 2

if __name__ == '__main__':
    raise SystemExit(main())
