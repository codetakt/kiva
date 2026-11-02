"""Controlled interpreter fixture for IEEE 4.2.4.2; never used by Kiva admission.

This finite consumer implements the declared examples, not an LRS grading duty.
Invalid/unknown leading parameters are ignored under an explicit fixture policy;
wrong-position list parameters remain literal text. No scoring is inferred from
HTTP status, and no string comparison policy is imposed on every producer.
"""
from collections import Counter
from decimal import Decimal, InvalidOperation
import re

COMPONENTS={'choice':{'a','b'},'sequencing':{'a','b'},'likert':{'a','b'},
            'matching':({'s1','s2'},{'t1','t2'}),'performance':{'p1','p2'}}
PARAMETER=re.compile(r'^\{([^={}]+)=([^{}]*)\}')
NUMBER=re.compile(r'[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\Z')


def parameters(value, allowed, stop=()):
    """Only consume the contiguous prefix at the declared scope."""
    found={}
    ignored=[]
    while match:=PARAMETER.match(value):
        name,argument=match.groups()
        if name in stop:
            break
        if name in allowed and (name=='lang' or argument in ('true','false')):
            found[name]=argument
        else:
            ignored.append((name,argument))
        value=value[match.end():]
    return value,found,ignored


def range_value(value):
    pieces=value.split('[:]')
    if len(pieces) not in (1,2) or not any(pieces):
        raise ValueError('fixture numeric syntax')
    if any(p and not NUMBER.fullmatch(p) for p in pieces):
        raise ValueError('fixture numeric syntax')
    try:
        limits=[Decimal(p) if p else None for p in pieces]
    except InvalidOperation as error:
        raise ValueError('fixture numeric syntax') from error
    low,high=(limits*2 if len(limits)==1 else limits)
    if low is not None and high is not None and low>high:
        raise ValueError('fixture reversed range')
    return low,high


def number_matches(pattern,response):
    low,high=range_value(pattern)
    rlow,rhigh=range_value(response)
    return ((low is None or rlow is not None and rlow>=low)
            and (high is None or rhigh is not None and rhigh<=high))


def items(value):
    values=value.split('[,]')
    if any(not item for item in values):
        raise ValueError('fixture requires nonempty example items')
    return values


def string_item(value,case_matters):
    text,metadata,ignored=parameters(value,{'lang'},stop=('case_matters','order_matters'))
    return (text if case_matters else text.casefold(),metadata.get('lang'),ignored)


def pattern_matches(kind,pattern,response):
    if not isinstance(pattern,str) or not isinstance(response,str):
        raise ValueError('fixture pattern and response must be Strings')
    if kind in ('fill-in','long-fill-in','performance'):
        allowed={'order_matters'}|({'case_matters'} if kind!='performance' else set())
        pattern,options,_=parameters(pattern,allowed,stop=('lang',))
        # Parameters describe correct-response patterns. The response can carry
        # its per-item language, but cannot override the pattern's list policy.
        case_matters=options.get('case_matters','false')=='true'
        order_matters=options.get('order_matters','true')=='true'
        left,right=items(pattern),items(response)
        if kind!='performance':
            def parsed(values):
                return [(text,language) for text,language,_ in
                        (string_item(value,case_matters) for value in values)]
            left,right=parsed(left),parsed(right)
            return left==right if order_matters else Counter(left)==Counter(right)
        def steps(values):
            result=[]
            for value in values:
                pair=value.split('[.]')
                if len(pair)!=2 or pair[0] not in COMPONENTS[kind]:
                    raise ValueError('fixture performance step')
                result.append(tuple(pair))
            return result
        left,right=steps(left),steps(right)
        if len(left)!=len(right):return False
        if not order_matters:
            left,right=sorted(left),sorted(right)
        for (lid,expected),(rid,actual) in zip(left,right):
            if lid!=rid:return False
            text,metadata,_=parameters(expected,{'lang'})
            try:range_value(text)
            except ValueError:
                # case_matters is not defined for performance. This fixture
                # selects case-sensitive performance Strings explicitly.
                if string_item(expected,True)[:2]!=string_item(actual,True)[:2]:return False
            else:
                if metadata:raise ValueError('lang is not a numeric-range parameter')
                if not number_matches(text,actual):return False
        return True
    if kind=='numeric':return number_matches(pattern,response)
    if kind=='true-false':
        if pattern not in ('true','false') or response not in ('true','false'):
            raise ValueError('fixture Boolean vocabulary')
        return pattern==response
    if kind=='other':return pattern==response
    if kind in ('choice','sequencing','likert'):
        left,right=items(pattern),items(response)
        if any(v not in COMPONENTS[kind] for v in left+right):raise ValueError('fixture component id')
        if kind=='likert' and (len(left)!=1 or len(right)!=1):raise ValueError('fixture single likert id')
        return Counter(left)==Counter(right) if kind=='choice' else left==right
    if kind=='matching':
        def pairs(value):
            result=[]
            for item in items(value):
                pair=item.split('[.]')
                if len(pair)!=2 or pair[0] not in COMPONENTS[kind][0] or pair[1] not in COMPONENTS[kind][1]:
                    raise ValueError('fixture matching pair')
                result.append(tuple(pair))
            return Counter(result)
        return pairs(pattern)==pairs(response)
    raise ValueError('fixture unknown interaction type')


def correct(kind,patterns,response):
    """Any COMPLETE pattern matches; never a union of partial pattern matches."""
    return any(pattern_matches(kind,pattern,response) for pattern in patterns)
