"""Author reference packet: hashlib + integer RSA, checked by OpenSSL (not Kiva/ring)."""
import base64,hashlib,json,subprocess,shutil,tempfile,sys
from pathlib import Path

workspace=Path(__file__).resolve().parents[2]
fixture=workspace/'tests/conformance/fixtures'
output=Path(sys.argv[1]) if len(sys.argv)>1 else fixture/'jws-provider.json'
temporary=tempfile.TemporaryDirectory(prefix='kiva-jws-reference-')
root=Path(temporary.name)
for bits,keyname in [(2047,'synthetic-2047-key.pem'),(2048,'synthetic-key.pem')]:
 key=fixture/'signing'/keyname
 subprocess.run(['openssl','rsa','-in',str(key),'-traditional','-outform','DER','-out',str(root/f'key-{bits}.der')],check=True,capture_output=True)
 for form,suffix in [('DER','der'),('PEM','pem')]:
  subprocess.run(['openssl','pkey','-in',str(key),'-pubout','-outform',form,'-out',str(root/(f'spki-{bits}.der' if form=='DER' else f'public-{bits}.pem'))],check=True,capture_output=True)
def b64(v):return base64.b64encode(v).decode()
def url(v):return base64.urlsafe_b64encode(v).decode().rstrip('=')
def der_one(raw):
 tag,n=raw[:2];at=2
 if n&128:
  size=n&127;n=int.from_bytes(raw[at:at+size]);at+=size
 return tag,raw[at:at+n],raw[at+n:]
def integers(raw):
 tag,body,tail=der_one(raw);assert tag==48 and not tail
 out=[]
 while body:
  tag,value,body=der_one(body);assert tag==2;out.append(int.from_bytes(value))
 return out
def encoded(algorithm,message,k):
 suffix={'RS256':'3031300d060960864801650304020105000420','RS384':'3041300d060960864801650304020205000430','RS512':'3051300d060960864801650304020305000440'}
 digest=getattr(hashlib,'sha'+algorithm[2:])(message).digest()
 t=bytes.fromhex(suffix[algorithm])+digest
 return b'\0\1'+b'\xff'*(k-3-len(t))+b'\0'+t
def check_openssl(bits,alg,message,signature):
 (root/'message.bin').write_bytes(message);(root/'signature.bin').write_bytes(signature)
 result=subprocess.run(['openssl','dgst','-sha'+alg[2:],'-verify',str(root/f'public-{bits}.pem'),'-signature',str(root/'signature.bin'),str(root/'message.bin')],capture_output=True)
 assert result.returncode in (0,1),(result.returncode,result.stderr)
 return result.returncode==0
payload={'actor':{'mbox':'mailto:rsa-boundary@example.org'},'verb':{'id':'https://example.org/rsa'},'object':{'id':'https://example.org/activity'}}
payload_bytes=json.dumps(payload,separators=(',',':')).encode()
rows=[];keys={}
for bits in (2047,2048):
 v=integers((root/f'key-{bits}.der').read_bytes());version,n,e,d,*_=v
 assert version==0 and n.bit_length()==bits and e==65537
 k=(bits+7)//8;keys[str(bits)]={'spki_der_base64':b64((root/f'spki-{bits}.der').read_bytes()),'modulus_hex':n.to_bytes(k,'big').hex(),'exponent':e}
 for alg in ('RS256','RS384','RS512'):
  header={'alg':alg,'kid':f'fixture{bits}'}
  h=url(json.dumps(header,separators=(',',':')).encode());message=f'{h}.{url(payload_bytes)}'.encode()
  em=encoded(alg,message,k);s=pow(int.from_bytes(em),d,n).to_bytes(k,'big')
  assert pow(int.from_bytes(s),e,n).to_bytes(k,'big')==em
  assert check_openssl(bits,alg,message,s)
  variants=[('valid',message,s,True)]
  if bits==2048:
   variants.extend([('signature-k-plus-one',message,b'\0'+s,False),('representative-n-minus-one',message,(n-1).to_bytes(k,'big'),False),('representative-n',message,n.to_bytes(k,'big'),False)])
   # Equivalent JSON with a different serialization is a different signing input.
   changed=f'{h}.{url(json.dumps(payload,indent=2).encode())}'.encode()
   variants.append(('reserialized-equivalent-payload',changed,s,False))
   end=em.index(b'\0',2)
   damaged={}
   a=bytearray(em);a[1]=2;damaged['wrong-block-type']=bytes(a)
   a=bytearray(em);a[10]=0x42;damaged['non-ff-padding']=bytes(a)
   a=bytearray(em);a[9]=0;damaged['seven-padding-octets']=bytes(a)
   a=bytearray(em);a[-1]^=1;damaged['wrong-full-digest']=bytes(a)
   a=bytearray(em);a[end+15]^=1;damaged['wrong-digestinfo-oid']=bytes(a)
   damaged['extra-digestinfo']=b'\0\1'+b'\xff'*(end-3)+b'\0'+em[end+1:]+b'\0'
   for cause,raw in damaged.items():
    assert len(raw)==k and int.from_bytes(raw)<n
    signature=pow(int.from_bytes(raw),d,n).to_bytes(k,'big')
    assert pow(int.from_bytes(signature),e,n).to_bytes(k,'big')==raw
    variants.append((cause,message,signature,False))
   # Preserve the representative when removing a leading zero octet. Search
   # changes only an allowed noncritical header and has a deterministic bound.
   for nonce in range(4096):
    header['nonce']=nonce
    short_message=f'{url(json.dumps(header,separators=(",",":")).encode())}.{url(payload_bytes)}'.encode()
    short_em=encoded(alg,short_message,k)
    signature=pow(int.from_bytes(short_em),d,n).to_bytes(k,'big')
    if signature[0]==0:
     variants.append(('leading-zero-signature-valid',short_message,signature,True))
     variants.append(('signature-k-minus-one-same-representative',short_message,signature[1:],False))
     break
   else:raise AssertionError('leading-zero search exhausted')
  for cause,msg,sig,primitive in variants:
   observed=check_openssl(bits,alg,msg,sig)
   # OpenSSL may left-pad short representatives, unlike RFC3447's k-octet
   # signature interface. Record its actual result without treating it as an
   # oracle for that explicit interface requirement.
   if cause!='signature-k-minus-one-same-representative':assert observed==primitive,(cause,alg,observed)
   row={'id':f'{bits}-{alg}-{cause}','bits':bits,'alg':alg,'cause':cause,'message_ascii':msg.decode(),'signature_base64url':url(sig),'signature_octets':len(sig),'openssl_verified':observed,'jws_permitted':bits>=2048 and primitive}
   if bits==2047:
    certificate=(fixture/'signing/synthetic-2047-certificate.der').read_bytes()
    protected=json.dumps({'alg':alg,'x5c':[b64(certificate)]},separators=(',',':')).encode()
    x5c_message=f'{url(protected)}.{url(payload_bytes)}'.encode()
    x5c_signature=pow(int.from_bytes(encoded(alg,x5c_message,k)),d,n).to_bytes(k,'big')
    assert check_openssl(bits,alg,x5c_message,x5c_signature)
    row['x5c_jws']=x5c_message.decode()+'.'+url(x5c_signature)
   if int.from_bytes(sig)<n:
    row['recovered_encoded_message_hex']=pow(int.from_bytes(sig),e,n).to_bytes(k,'big').hex()
   rows.append(row)
jose=[]
certificate=(fixture/'signing/synthetic-certificate.der').read_bytes()
for alg in ('RS256','RS384','RS512'):
 default={'alg':alg,'kid':'fixture2048'}
 header=json.dumps(default,separators=(',',':'))
 def changed(**fields):return json.dumps(default|fields,separators=(',',':')).encode()
 forms=[('header-valid',header.encode(),payload_bytes,True,'required'),
  ('header-alg-escaped',header.replace(alg,'\\u0052'+alg[1:]).encode(),payload_bytes,True,'decoded-string'),
  ('header-kid-escaped',header.replace('fixture2048','fixture204\\u0038').encode(),payload_bytes,True,'decoded-string'),
  ('header-kid-case',changed(kid='Fixture2048'),payload_bytes,True,'D10-unknown-not-evaluated'),
  ('header-noncritical',changed(extension=True),payload_bytes,True,'required'),
  ('header-typ-cty-case',changed(typ='JWT',cty='Application/JSON'),payload_bytes,True,'optional-application-hints'),
  ('header-nonbmp-literal',(header[:-1]+',"𝄞":1}').encode(),payload_bytes,True,'unicode-capable'),
  ('header-nonbmp-escaped',(header[:-1]+',"\\ud834\\udd1e":1}').encode(),payload_bytes,True,'unicode-capable'),
  ('header-nonbmp-distinct',(header[:-1]+',"𝄞":1,"\\ud834\\udd1f":2}').encode(),payload_bytes,True,'unicode-capable'),
  ('header-nonbmp-duplicate',(header[:-1]+',"𝄞":1,"\\ud834\\udd1e":2}').encode(),payload_bytes,False,'D11-reject'),
  ('header-alg-duplicate-same',(header[:-1]+',"alg":"'+alg+'"}').encode(),payload_bytes,False,'D11-reject'),
  ('header-alg-duplicate-conflict',(header[:-1]+',"alg":"none"}').encode(),payload_bytes,False,'D11-reject'),
  ('header-alg-escaped-duplicate',(header[:-1]+',"\\u0061lg":"'+alg+'"}').encode(),payload_bytes,False,'D11-reject'),
  ('header-critical-unknown',changed(crit=['extension'],extension=True),payload_bytes,False,'required'),
  ('header-critical-empty',changed(crit=[]),payload_bytes,False,'receiver-MAY-selected-reject'),
  ('header-critical-missing',changed(crit=['extension']),payload_bytes,False,'receiver-MAY-selected-reject'),
  ('header-array',b'[]',payload_bytes,False,'required'),('header-null',b'null',payload_bytes,False,'required'),
  ('header-invalid-utf8',b'\xff',payload_bytes,False,'required'),('header-invalid-json',b'{',payload_bytes,False,'required'),
  ('header-missing-alg',b'{"kid":"fixture2048"}',payload_bytes,False,'required'),
  ('header-kid-type',changed(kid=[]),payload_bytes,False,'required'),
  ('x5c-standard-base64',changed(x5c=[b64(certificate)]),payload_bytes,True,'required'),
  ('x5c-url-alphabet',changed(x5c=[b64(certificate).replace('+','-').replace('/','_')]),payload_bytes,False,'required'),
  ('x5c-trailing-der',changed(x5c=[b64(certificate+b'\0')]),payload_bytes,False,'required'),
  ('x5c-malformed-der',changed(x5c=[b64(b'\x30\x00')]),payload_bytes,False,'required'),
  ('x5c-empty',changed(x5c=[]),payload_bytes,False,'required'),
  ('x5c-nonstring',changed(x5c=[None]),payload_bytes,False,'required')]
 for bad in (alg.lower(),'none','HS256','ES256'):
  forms.append(('header-alg-'+bad,changed(alg=bad),payload_bytes,False,'required'))
 for field in ('actor','verb','object'):
  body={key:value for key,value in payload.items() if key!=field}
  forms.append(('payload-missing-'+field,header.encode(),json.dumps(body,separators=(',',':')).encode(),False,'required'))
 for name,body in [('empty',b''),('invalid-json',b'{'),('invalid-utf8',b'\xff'),('null',b'null')]:
  forms.append(('payload-'+name,header.encode(),body,False,'required'))
 for name,protected,body,accepted,role in forms:
  msg=f'{url(protected)}.{url(body)}'.encode();sig=pow(int.from_bytes(encoded(alg,msg,256)),d,n).to_bytes(256,'big')
  assert check_openssl(2048,alg,msg,sig)
  compact=msg.decode()+'.'+url(sig)
  jose.append({'id':alg+'-'+name,'jws':compact,'accepted':accepted,'role':role,'openssl_signed_message_verified':True})
 # Exact original components for compact-interface-only failures.
 base=jose[-len(forms)]['jws'];segments=base.split('.')
 bad=[('two-segments','.'.join(segments[:2])),('four-segments',base+'.AAAA')]
 for index in range(3):
  for change in ('padding','whitespace'):
   parts=segments.copy();parts[index]+='=' if change=='padding' else ' '
   bad.append((f'{change}-segment-{index}','.'.join(parts)))
 parts=segments.copy();parts[2]=parts[2].replace('-','+').replace('_','/');assert parts!=segments
 bad.append(('standard-alphabet-compact','.'.join(parts)))
 alphabet='ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_'
 parts=segments.copy();parts[2]=parts[2][:-1]+alphabet[alphabet.index(parts[2][-1])+1]
 bad.append(('nonzero-unused-bits','.'.join(parts)))
 for name,compact in bad:
  jose.append({'id':alg+'-'+name,'jws':compact,'accepted':False,'role':'canonical-decoder-policy' if name=='nonzero-unused-bits' else 'required','openssl_signed_message_verified':True})
openssl=Path(shutil.which('openssl')).resolve()
report={'scope':'independent fixture producer; author evidence, not independent review; test-only keys','openssl':{'path':str(openssl),'sha256':hashlib.sha256(openssl.read_bytes()).hexdigest(),'version':subprocess.check_output(['openssl','version'],text=True).strip()},'keys':keys,'payload':payload,'vectors':rows,'padding_boundary':'With >=2048-bit RSA and RS256/384/512 DigestInfo, valid padding is >=170 octets; no allowed-key valid-PS8 vector exists. The seven-octet negative deliberately inserts an early separator and retains the remaining full block, so it is not claimed a positive PS8 boundary proof.'}
report['jose_vectors']=jose
def der_wrap(tag,body):
 length=len(body)
 size=length.to_bytes((length.bit_length()+7)//8,'big')
 return bytes([tag])+(bytes([length]) if length<128 else bytes([128+len(size)])+size)+body
def children(raw):
 tag,body,tail=der_one(raw);assert tag==48 and not tail
 result=[]
 while body:
  tag,value,body=der_one(body);result.append((tag,value))
 return result
outer=children(certificate);fields=children(der_wrap(*outer[0]));offset=int(fields[0][0]==160)
fields[offset+3]=(48,der_wrap(24,b'20261027000000.1234567891Z')+der_wrap(24,b'20261027000000.7000000001Z'))
tbs=der_wrap(48,b''.join(der_wrap(*part) for part in fields))
cert_sig=pow(int.from_bytes(encoded('RS256',tbs,256)),d,n).to_bytes(256,'big')
assert check_openssl(2048,'RS256',tbs,cert_sig)
precise=der_wrap(48,tbs+der_wrap(*outer[1])+der_wrap(3,b'\0'+cert_sig))
fraction_vectors=[]
for alg in ('RS256','RS384','RS512'):
 protected=json.dumps({'alg':alg,'x5c':[b64(precise)]},separators=(',',':')).encode()
 msg=f'{url(protected)}.{url(payload_bytes)}'.encode()
 sig=pow(int.from_bytes(encoded(alg,msg,256)),d,n).to_bytes(256,'big')
 assert check_openssl(2048,alg,msg,sig)
 fraction_vectors.append({'id':alg+'-original-subnanosecond-certificate','jws':msg.decode()+'.'+url(sig),
  'role':'original-certificate-time','openssl_signed_message_verified':True})
report['fractional_certificate']={'der_base64':b64(precise),'original_tbs_sha256':hashlib.sha256(tbs).hexdigest(),
 'openssl_original_certificate_signature_verified':True,'vectors':fraction_vectors,
 'clock_cases':[{'time':'2026-10-27T00:00:00'+fraction+'Z','accepted':accepted} for fraction,accepted in
  [('.123456789',False),('.123456790',True),('.7',True),('.700000001',False)]]}
report['input_sha256']={str(path.relative_to(workspace)):hashlib.sha256(path.read_bytes()).hexdigest() for path in [Path(__file__).resolve(),fixture/'signing/synthetic-key.pem',fixture/'signing/synthetic-2047-key.pem',fixture/'signing/synthetic-certificate.der',fixture/'signing/synthetic-2047-certificate.der']}
output.write_text(json.dumps(report,indent=2)+'\n')
print(len(rows),'RSA and',len(jose),'JOSE vectors produced with original-byte OpenSSL observations')
