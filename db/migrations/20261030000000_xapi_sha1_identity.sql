-- Agent SHA-1 IFIs identify digest bytes, independent of hex letter case.
-- Preserve received JSON, document content, timestamps and ETags. Only the
-- derived identity keys change. Slice bytea keys by their structural prefix:
-- arbitrary document IDs and Agent names may include NUL and fake key labels.
CREATE FUNCTION pg_temp.kiva_sha1_document_key(kind text, activity text, old_key bytea)
RETURNS bytea LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    prefix text := 'kind:' || octet_length(kind)::text || ':' || kind || '|';
    prefix_bytes bytea;
    agent_part bytea;
BEGIN
    IF kind NOT IN ('state', 'agent-profile') THEN
        RETURN old_key;
    END IF;
    IF kind = 'state' THEN
        prefix := prefix || 'activity_id:' || octet_length(activity)::text || ':' || activity || '|';
    END IF;
    prefix_bytes := convert_to(prefix, 'UTF8');
    IF substring(old_key FROM 1 FOR octet_length(prefix_bytes)) <> prefix_bytes THEN
        RETURN old_key;
    END IF;
    -- agent:53:mbox_sha1sum:<40 hex digits>| occupies exactly 63 bytes.
    agent_part := substring(old_key FROM octet_length(prefix_bytes) + 1 FOR 63);
    IF encode(agent_part, 'escape') !~ '^agent:53:mbox_sha1sum:[0-9A-Fa-f]{40}\|$' THEN
        RETURN old_key;
    END IF;
    RETURN prefix_bytes || convert_to(lower(encode(agent_part, 'escape')), 'UTF8') ||
        substring(old_key FROM octet_length(prefix_bytes) + 64);
END
$$;

-- Preflight before any data change. Previously split scopes may contain two
-- documents at one logical address; choosing a winner would silently lose data.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM kiva.xapi_documents
        GROUP BY tenant_id, resource_kind,
            pg_temp.kiva_sha1_document_key(resource_kind, activity_id, document_key)
        HAVING count(*) > 1
    ) THEN
        RAISE EXCEPTION 'SHA-1 Agent identity collision in xAPI documents'
            USING HINT = 'Reconcile documents with the same tenant, scope and document ID across hex-case spellings before retrying. No document is automatically discarded.';
    END IF;
END
$$;

UPDATE kiva.xapi_statements
SET actor_ifi = convert_to(lower(encode(actor_ifi, 'escape')), 'UTF8')
WHERE encode(actor_ifi, 'escape') ~ '^mbox_sha1sum:[0-9A-Fa-f]{40}$';

UPDATE kiva.xapi_documents
SET document_key = pg_temp.kiva_sha1_document_key(resource_kind, activity_id, document_key)
WHERE document_key <> pg_temp.kiva_sha1_document_key(resource_kind, activity_id, document_key);
