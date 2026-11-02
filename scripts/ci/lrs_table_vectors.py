"""Lossless accepted-table fixtures, including raw MIME omission witnesses."""
import copy
import hashlib
from email.parser import BytesParser
import email.policy

from lrs_preservation import dump_json_members, load_json_members, load_yaml_numbers


def accepted_vectors(root):
    model_path=root/'tests/conformance/models/lrs-tables.yaml'
    models=load_yaml_numbers(model_path.read_text())
    model_digest=hashlib.sha256(model_path.read_bytes()).hexdigest()
    placements=[]
    for place in models['placements']:
        name=place['model']+'-'+(place['pointer'].strip('/').replace('/','-') or 'root')
        placements.append((name,place['model'],place['pointer']))
    files=['tables','behaviors']
    for name in files:
        path=root/f'tests/conformance/http/generated/{name}.yaml'
        document=load_yaml_numbers(path.read_text())
        digest=hashlib.sha256(path.read_bytes()).hexdigest()
        for case in document['cases']:
            if name=='behaviors' and case['id'] not in ('attachment-two-file-urls','attachment-two-multipart'):
                continue
            first=case.get('steps',[case])[0]
            request=first['request']
            if request['path']!='/statements' or first['expect'].get('status') not in (200,204):
                continue
            if name=='tables':
                candidates=[p for p in placements if case['id'].startswith(p[0]+'-')]
                label,model,pointer=max(candidates,key=lambda p:len(p[0]))
                source={'model':model,'placement':pointer,'condition':case['id'][len(label)+1:],
                        'source_anchor':models['models'][model]['source_anchor']}
            else:
                source={'model':'attachment','placement':'/attachments','condition':case['id'],
                        'source_anchor':models['models']['attachment']['source_anchor']}
            yield case['id'],request,{**source,'case_file':str(path.relative_to(root)),
                'case_file_sha256':digest,
                'model_sha256':model_digest,
                'source_revision':models['source_revision']}


def statement_template(request):
    if isinstance(request.get('body'),dict):
        return copy.deepcopy(request['body']),None
    raw=bytes.fromhex(request['raw_body_hex'])
    content_type=request['headers']['Content-Type']
    message=BytesParser(policy=email.policy.HTTP).parsebytes(b'Content-Type: '+content_type.encode()+b'\r\n\r\n'+raw)
    parts=list(message.iter_parts())
    if not message.is_multipart() or len(parts)<2 or parts[0].get_content_type()!='application/json':
        raise ValueError('accepted table MIME fixture does not have a JSON first part')
    # Replace only the first JSON entity; framing, part headers and raw payloads
    # remain the authored fixture bytes, including two distinct attachment parts.
    boundary=message.get_boundary().encode()
    start=raw.index(b'\r\n\r\n')+4
    end=raw.index(b'\r\n--'+boundary,start)
    value=load_json_members(raw[start:end])
    if not isinstance(value,dict):
        raise ValueError('accepted table fixture must be a single Statement')
    # Core Statement members are unique in these positive fixtures. Make the
    # root mutable without retaining MemberObject's original pair list; nested
    # owner data still keeps its lossless member representation.
    if len(value.pairs)!=len(value):
        raise ValueError('positive Statement fixture has duplicate root members')
    return dict(value),(raw[:start],raw[end:],content_type)


def write_payload(value, envelope):
    if envelope is None:
        return {'body':value}
    before,after,content_type=envelope
    return {'raw':before+dump_json_members(value).encode()+after,
            'headers':{'Content-Type':content_type}}
