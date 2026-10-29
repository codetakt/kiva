"""Field-specific DC-IRL observations, independent of receiver admission policy.

No blanket urn ban, identifier fetch, or semantic inspection of owner extensions.
"""
from lrs_preservation import locator_validity


def locator_observations(document, role='statement', evidence=None):
    checks=[]
    def add(path,value,kind='locator'):
        valid=locator_validity(value,evidence) if kind=='locator' else isinstance(value,str)
        checks.append({'pointer':path,'kind':kind,'valid':valid,'value':value})
    def account(value,path):
        if not isinstance(value,dict):
            checks.append({'pointer':path,'kind':'account-object','valid':False,'value':value})
            return
        add(path+'/homePage',value.get('homePage'))
        add(path+'/name',value.get('name'),'account-name')
    def actor(value,path):
        if not isinstance(value,dict):return
        if 'account' in value:account(value['account'],path+'/account')
        if isinstance(value.get('member'),list):
            for index,member in enumerate(value['member']):actor(member,f'{path}/member/{index}')
    def activity(value,path):
        if not isinstance(value,dict):return
        definition=value.get('definition')
        if isinstance(definition,dict) and 'moreInfo' in definition:
            add(path+'/definition/moreInfo',definition['moreInfo'])
    def statement(value,path):
        if not isinstance(value,dict):return
        for key in ('actor','authority'):actor(value.get(key),path+'/'+key)
        obj=value.get('object')
        if isinstance(obj,dict):
            kind=obj.get('objectType','Activity')
            if kind=='SubStatement':statement(obj,path+'/object')
            elif kind in ('Agent','Group'):actor(obj,path+'/object')
            elif kind=='Activity':activity(obj,path+'/object')
        context=value.get('context',{})
        if isinstance(context,dict):
            for key in ('instructor','team'):actor(context.get(key),path+'/context/'+key)
            for collection,key in (('contextAgents','agent'),('contextGroups','group')):
                for index,entry in enumerate(context.get(collection,[])):
                    if isinstance(entry,dict):actor(entry.get(key),f'{path}/context/{collection}/{index}/{key}')
            activities=context.get('contextActivities',{})
            if isinstance(activities,dict):
                for category in ('parent','grouping','category','other'):
                    entries=activities.get(category,[])
                    if isinstance(entries,dict):activity(entries,f'{path}/context/contextActivities/{category}')
                    elif isinstance(entries,list):
                        for index,item in enumerate(entries):activity(item,f'{path}/context/contextActivities/{category}/{index}')
        for index,attachment in enumerate(value.get('attachments',[])):
            if isinstance(attachment,dict) and 'fileUrl' in attachment:
                add(f'{path}/attachments/{index}/fileUrl',attachment['fileUrl'])
    if role=='person':
        for index,item in enumerate(document.get('account',[])):account(item,f'/account/{index}')
    elif role=='agent':actor(document,'')
    elif role=='statement':statement(document,'')
    else:raise ValueError('unknown locator observation role')
    return checks
