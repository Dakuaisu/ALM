-- Local development schema, continued: legacy import integrity (OPEN-004: the authority no
-- longer trusts the device's mapped plan alone). The device uploads the raw legacy file with
-- the plan (`"source"`, UTF-8 JSON text). The authority stores it Owner-only
-- (`legacy_sources`, RLS, no client grants), computes its SHA-256 with pgcrypto and refuses a
-- plan whose digest differs (ALM05) or that comes without its file. Each manifest entry keeps
-- the record's place in the stored file (`"source"`: a JSON pointer such as `/sales/3`; empty
-- when `migrate()` created it). `alm-cli verify-legacy-import` re-parses the stored file with
-- the core and compares its plan record by record with the manifest. Synthetic files only.

create extension if not exists pgcrypto with schema extensions;

create table public.legacy_sources (
    digest text primary key,
    source text not null,
    stored_at timestamptz not null default now()
);
alter table public.legacy_sources enable row level security;

create or replace function public.import_legacy(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_id bigint;
    v_entries jsonb := '[]';
    v_q jsonb;
    v_r jsonb;
    v_ref jsonb;
    v_missing jsonb;
    v_existing public.legacy_records%rowtype;
    v_aid text;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'import_legacy', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if jsonb_typeof(p_terms -> 'digest') is distinct from 'string'
        or jsonb_typeof(p_terms -> 'version') is distinct from 'string'
        or jsonb_typeof(p_terms -> 'records') is distinct from 'array'
        or jsonb_typeof(p_terms -> 'quarantine') is distinct from 'array' then
        perform alm_private.invalid('plan');
    end if;
    -- The source file must come with the plan and hash to its digest.
    if jsonb_typeof(p_terms -> 'source') is distinct from 'string'
        or p_terms ->> 'source' = '' then
        raise exception 'invalid_terms: no source file' using errcode = 'ALM05';
    end if;
    if encode(extensions.digest(convert_to(p_terms ->> 'source', 'UTF8'), 'sha256'), 'hex')
            <> p_terms ->> 'digest' then
        raise exception 'invalid_terms: source digest' using errcode = 'ALM05';
    end if;
    perform pg_advisory_xact_lock(hashtextextended('legacy_import', 0));
    insert into public.legacy_sources (digest, source)
        values (p_terms ->> 'digest', p_terms ->> 'source')
        on conflict (digest) do nothing;
    select id into v_id from public.legacy_manifests
        where digest = p_terms ->> 'digest' and version = p_terms ->> 'version';
    if found then
        return alm_private.record(p_request_id, 'import_legacy', p_terms,
            jsonb_build_object('created', v_id));
    end if;
    v_id := coalesce((select max(id) from public.legacy_manifests), 0) + 1;
    for v_q in select e from jsonb_array_elements(p_terms -> 'quarantine') with ordinality t(e, n)
        order by n
    loop
        v_entries := v_entries || jsonb_build_array(jsonb_build_object(
            'kind', v_q ->> 'kind', 'legacy_id', v_q ->> 'legacy_id', 'authority_id', null,
            'status', 'quarantined', 'reason', v_q ->> 'reason',
            'source', coalesce(v_q ->> 'source', '')));
    end loop;
    for v_r in select e from jsonb_array_elements(p_terms -> 'records') with ordinality t(e, n)
        order by n
    loop
        select * into v_existing from public.legacy_records
            where kind = v_r ->> 'kind' and legacy_id = v_r ->> 'legacy_id';
        if found then
            if v_existing.body = v_r -> 'body' then
                v_entries := v_entries || jsonb_build_array(jsonb_build_object(
                    'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                    'authority_id', v_existing.authority_id, 'status', 'already', 'reason', '',
                    'source', coalesce(v_r ->> 'source', '')));
            else
                v_entries := v_entries || jsonb_build_array(jsonb_build_object(
                    'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                    'authority_id', null, 'status', 'quarantined',
                    'reason', 'differs from the imported record ' || v_existing.authority_id,
                    'source', coalesce(v_r ->> 'source', '')));
            end if;
            continue;
        end if;
        v_missing := null;
        for v_ref in select e from jsonb_array_elements(coalesce(v_r -> 'refs', '[]'))
            with ordinality t(e, n) order by n
        loop
            if not exists (select 1 from public.legacy_records
                    where kind = v_ref ->> 0 and legacy_id = v_ref ->> 1) then
                v_missing := v_ref;
                exit;
            end if;
        end loop;
        if v_missing is not null then
            v_entries := v_entries || jsonb_build_array(jsonb_build_object(
                'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                'authority_id', null, 'status', 'quarantined',
                'reason', 'unresolved reference ' || (v_missing ->> 0) || ' ' || (v_missing ->> 1),
                'source', coalesce(v_r ->> 'source', '')));
            continue;
        end if;
        v_aid := (v_r ->> 'kind') || '-'
            || ((select count(*) from public.legacy_records where kind = v_r ->> 'kind') + 1);
        insert into public.legacy_records (authority_id, kind, legacy_id, body, manifest)
            values (v_aid, v_r ->> 'kind', v_r ->> 'legacy_id', v_r -> 'body', v_id);
        v_entries := v_entries || jsonb_build_array(jsonb_build_object(
            'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
            'authority_id', v_aid, 'status', 'imported', 'reason', '',
            'source', coalesce(v_r ->> 'source', '')));
    end loop;
    insert into public.legacy_manifests (id, digest, version, entries)
        values (v_id, p_terms ->> 'digest', p_terms ->> 'version', v_entries);
    perform alm_private.bump();
    return alm_private.record(p_request_id, 'import_legacy', p_terms,
        jsonb_build_object('created', v_id));
end;
$$;

revoke all on all tables in schema public from anon, authenticated;
revoke all on function public.import_legacy(text, jsonb) from public, anon;
grant execute on function public.import_legacy(text, jsonb) to authenticated, service_role;
