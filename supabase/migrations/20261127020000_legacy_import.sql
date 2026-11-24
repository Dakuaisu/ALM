-- Local development schema, continued: the legacy import rehearsal (R-8, Phase 4 groundwork;
-- synthetic files only, OPEN-002 / OPEN-006). `import_legacy` applies the plan the core maps
-- from an HTML backup file (`alm_core::legacy::import::plan`) with the core's rule
-- (`legacy::import::apply`, checked by the differential suite):
--
-- {"digest", "version", "records": [{"kind", "legacy_id", "body", "refs": [[kind, legacy_id]]}],
--  "quarantine": [{"kind", "legacy_id", "reason"}]}
--
-- * The same file (digest) and mapping version again returns the earlier manifest and changes
--   nothing.
-- * Otherwise a new manifest lists every record: the mapping's quarantine first, then each
--   record in order: already imported with the same body (mapped to that authority record),
--   already imported with another body (quarantined, never merged), a reference to a record no
--   import placed (quarantined), or imported as `<kind>-<n>`.
-- Returns {"created": manifest id}. Nothing reaches the posting tables or the invoice and GST
-- series: the import is staged for reconciliation. Owner only (OPEN-004); the rows travel in
-- the Owner book only.

create table public.legacy_manifests (
    id bigint primary key,
    digest text not null,
    version text not null,
    entries jsonb not null,
    unique (digest, version)
);
alter table public.legacy_manifests enable row level security;

create table public.legacy_records (
    seq bigint generated always as identity,
    authority_id text primary key,
    kind text not null,
    legacy_id text not null,
    body jsonb not null,
    manifest bigint not null references public.legacy_manifests (id) deferrable initially deferred,
    unique (kind, legacy_id)
);
alter table public.legacy_records enable row level security;

create function public.import_legacy(p_request_id text, p_terms jsonb)
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
    perform pg_advisory_xact_lock(hashtextextended('legacy_import', 0));
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
            'status', 'quarantined', 'reason', v_q ->> 'reason'));
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
                    'authority_id', v_existing.authority_id, 'status', 'already', 'reason', ''));
            else
                v_entries := v_entries || jsonb_build_array(jsonb_build_object(
                    'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
                    'authority_id', null, 'status', 'quarantined',
                    'reason', 'differs from the imported record ' || v_existing.authority_id));
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
                'reason', 'unresolved reference ' || (v_missing ->> 0) || ' ' || (v_missing ->> 1)));
            continue;
        end if;
        v_aid := (v_r ->> 'kind') || '-'
            || ((select count(*) from public.legacy_records where kind = v_r ->> 'kind') + 1);
        insert into public.legacy_records (authority_id, kind, legacy_id, body, manifest)
            values (v_aid, v_r ->> 'kind', v_r ->> 'legacy_id', v_r -> 'body', v_id);
        v_entries := v_entries || jsonb_build_array(jsonb_build_object(
            'kind', v_r ->> 'kind', 'legacy_id', v_r ->> 'legacy_id',
            'authority_id', v_aid, 'status', 'imported', 'reason', ''));
    end loop;
    insert into public.legacy_manifests (id, digest, version, entries)
        values (v_id, p_terms ->> 'digest', p_terms ->> 'version', v_entries);
    perform alm_private.bump();
    return alm_private.record(p_request_id, 'import_legacy', p_terms,
        jsonb_build_object('created', v_id));
end;
$$;

-- The book carries the imports.
create function alm_private.legacy_rows(p_out jsonb)
returns jsonb
language sql
stable
set search_path = ''
as $$
    select jsonb_set(p_out, '{rows,owner,book,legacy}', jsonb_build_object(
        'manifests', coalesce((select jsonb_agg(jsonb_build_object('id', id, 'digest', digest,
                'version', version, 'entries', entries) order by id)
            from public.legacy_manifests), '[]'),
        'records', coalesce((select jsonb_agg(jsonb_build_object('authority_id', authority_id,
                'kind', kind, 'legacy_id', legacy_id, 'body', body, 'manifest', manifest)
                order by seq) from public.legacy_records), '[]')));
$$;

create or replace function public.pull_projection(p_checkpoint bigint)
returns jsonb
language plpgsql
security definer
stable
set search_path = ''
as $$
declare
    v_out jsonb := alm_private.pull_projection_body(p_checkpoint);
begin
    if v_out ->> 'role' = 'owner' and jsonb_typeof(v_out -> 'rows') = 'object' then
        v_out := jsonb_set(v_out, '{rows,owner,book}', alm_private.owner_book());
        v_out := alm_private.crate_rows(v_out);
        v_out := alm_private.customer_contacts(v_out);
        v_out := alm_private.activity_rows(v_out);
        v_out := alm_private.legacy_rows(v_out);
    end if;
    return v_out;
end;
$$;

revoke all on all tables in schema public from anon, authenticated;
revoke all on all sequences in schema public from anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.import_legacy(text, jsonb) from public, anon;
grant execute on function public.import_legacy(text, jsonb) to authenticated, service_role;
revoke all on function public.pull_projection(bigint) from public, anon;
grant execute on function public.pull_projection(bigint) to authenticated, service_role;
