-- Local development schema, continued: quarantine resolution for legacy imports, following the
-- core (`legacy::resolve::resolve`, checked by the differential suite). Owner only (OPEN-004);
-- never automatic. `resolve_legacy_quarantine` takes
--
-- {"manifest", "entry" (0-based position in the manifest), "kind", "legacy_id",
--  "decision": "import_corrected" | "keep_excluded", "body" (the corrected record as the core
--  mapped it; null to keep it excluded), "reason"}
--
-- and records one decision per quarantined entry, with its reason: keep the record excluded,
-- or stage the corrected record as a new authority record `<kind>-<n>` (refused when a record
-- with that kind and legacy ID is already imported or when it refers to a record no import
-- placed; the references are computed here from the body, as the core does). A decision is
-- never replaced. Rejections (ALM05): not a quarantined entry of that manifest with that kind
-- and legacy ID, already decided, no reason, a body with "keep excluded" or none with
-- "import as corrected", already imported, an unresolved reference.

create table public.legacy_resolutions (
    seq bigint generated always as identity,
    manifest bigint not null references public.legacy_manifests (id),
    entry integer not null,
    kind text not null,
    legacy_id text not null,
    decision text not null check (decision in ('import_corrected', 'keep_excluded')),
    reason text not null check (length(reason) > 0),
    authority_id text references public.legacy_records (authority_id),
    primary key (manifest, entry)
);
alter table public.legacy_resolutions enable row level security;

-- `legacy::import::record_refs`: the records a mapped body refers to, in order, without
-- repeats; a stock entry refers to its product first.
create function alm_private.legacy_refs(p_kind text, p_legacy text, p_body jsonb)
returns jsonb
language plpgsql
immutable
set search_path = ''
as $$
declare
    v_refs jsonb := '[]';
    v_pairs jsonb := '[]';
    v_line jsonb;
    v_comp jsonb;
    v_pair jsonb;
    v_key text;
begin
    if p_kind = 'stock' and p_legacy not like 'n:%' then
        v_pairs := v_pairs || jsonb_build_array(jsonb_build_array('product', p_legacy));
    end if;
    if p_kind in ('sale', 'payment', 'gst_bill') then
        v_pairs := v_pairs || jsonb_build_array(jsonb_build_array('customer',
            case when jsonb_typeof(p_body -> 'customerId') = 'string'
                then p_body ->> 'customerId' else '' end));
    elsif p_kind = 'purchase_order' then
        v_pairs := v_pairs || jsonb_build_array(jsonb_build_array('supplier',
            case when jsonb_typeof(p_body -> 'supId') = 'string'
                then p_body ->> 'supId' else '' end));
    end if;
    if p_kind = 'stock_log' then
        v_key := case when jsonb_typeof(p_body -> 'key') = 'string' then p_body ->> 'key' else '' end;
        if v_key not like 'n:%' then
            v_pairs := v_pairs || jsonb_build_array(jsonb_build_array('product', v_key));
        end if;
    end if;
    for v_line in
        select e from jsonb_array_elements(case when jsonb_typeof(p_body -> 'items') = 'array'
                then p_body -> 'items' else '[]' end) with ordinality t(e, n)
        union all
        select e from jsonb_array_elements(case when jsonb_typeof(p_body -> 'components') = 'array'
                then p_body -> 'components' else '[]' end) with ordinality t(e, n)
    loop
        v_pairs := v_pairs || jsonb_build_array(jsonb_build_array('product',
            case when jsonb_typeof(v_line -> 'pid') = 'string' then v_line ->> 'pid' else '' end));
        for v_comp in select e from jsonb_array_elements(
                case when jsonb_typeof(v_line -> 'components') = 'array'
                    then v_line -> 'components' else '[]' end) with ordinality t(e, n)
        loop
            v_pairs := v_pairs || jsonb_build_array(jsonb_build_array('product',
                case when jsonb_typeof(v_comp -> 'pid') = 'string'
                    then v_comp ->> 'pid' else '' end));
        end loop;
    end loop;
    for v_pair in select e from jsonb_array_elements(v_pairs) with ordinality t(e, n) order by n loop
        if v_pair ->> 1 <> '' and not exists (
                select 1 from jsonb_array_elements(v_refs) r where r = v_pair) then
            v_refs := v_refs || jsonb_build_array(v_pair);
        end if;
    end loop;
    return v_refs;
end;
$$;

create function public.resolve_legacy_quarantine(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
    v_entry jsonb;
    v_reason text := btrim(coalesce(p_terms ->> 'reason', ''), E' \t\r\n');
    v_body jsonb := p_terms -> 'body';
    v_ref jsonb;
    v_existing text;
    v_aid text;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'resolve_legacy_quarantine', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    perform pg_advisory_xact_lock(hashtextextended('legacy_import', 0));
    if jsonb_typeof(p_terms -> 'entry') is distinct from 'number'
        or (p_terms ->> 'entry')::bigint < 0 then
        perform alm_private.invalid('entry');
    end if;
    v_entry := (select entries -> ((p_terms ->> 'entry')::integer) from public.legacy_manifests
        where id = (p_terms ->> 'manifest')::bigint);
    if v_entry is null or v_entry ->> 'status' <> 'quarantined'
        or v_entry ->> 'kind' is distinct from p_terms ->> 'kind'
        or v_entry ->> 'legacy_id' is distinct from p_terms ->> 'legacy_id' then
        raise exception 'invalid_terms: not quarantined' using errcode = 'ALM05';
    end if;
    if exists (select 1 from public.legacy_resolutions
            where manifest = (p_terms ->> 'manifest')::bigint
                and entry = (p_terms ->> 'entry')::integer) then
        raise exception 'invalid_terms: already resolved' using errcode = 'ALM05';
    end if;
    if v_reason = '' then
        raise exception 'invalid_terms: reason' using errcode = 'ALM05';
    end if;
    if p_terms ->> 'decision' = 'keep_excluded'
            and (v_body is null or jsonb_typeof(v_body) = 'null') then
        v_aid := null;
    elsif p_terms ->> 'decision' = 'import_corrected' and jsonb_typeof(v_body) = 'object' then
        select authority_id into v_existing from public.legacy_records
            where kind = p_terms ->> 'kind' and legacy_id = p_terms ->> 'legacy_id';
        if found then
            raise exception 'invalid_terms: already imported as %', v_existing
                using errcode = 'ALM05';
        end if;
        for v_ref in select e from jsonb_array_elements(alm_private.legacy_refs(
                p_terms ->> 'kind', p_terms ->> 'legacy_id', v_body)) with ordinality t(e, n)
            order by n
        loop
            if not exists (select 1 from public.legacy_records
                    where kind = v_ref ->> 0 and legacy_id = v_ref ->> 1) then
                raise exception 'invalid_terms: unresolved reference % %', v_ref ->> 0,
                    v_ref ->> 1 using errcode = 'ALM05';
            end if;
        end loop;
        v_aid := (p_terms ->> 'kind') || '-'
            || ((select count(*) from public.legacy_records where kind = p_terms ->> 'kind') + 1);
        insert into public.legacy_records (authority_id, kind, legacy_id, body, manifest)
            values (v_aid, p_terms ->> 'kind', p_terms ->> 'legacy_id', v_body,
                (p_terms ->> 'manifest')::bigint);
    else
        raise exception 'invalid_terms: decision and body' using errcode = 'ALM05';
    end if;
    insert into public.legacy_resolutions (manifest, entry, kind, legacy_id, decision, reason,
            authority_id)
        values ((p_terms ->> 'manifest')::bigint, (p_terms ->> 'entry')::integer,
            p_terms ->> 'kind', p_terms ->> 'legacy_id', p_terms ->> 'decision', v_reason, v_aid);
    perform alm_private.bump();
    return alm_private.record(p_request_id, 'resolve_legacy_quarantine', p_terms,
        '{"done": true}');
end;
$$;

-- The book carries the decisions beside the manifests and records.
create or replace function alm_private.legacy_rows(p_out jsonb)
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
                order by seq) from public.legacy_records), '[]'),
        'resolutions', coalesce((select jsonb_agg(jsonb_build_object('manifest', manifest,
                'entry', entry, 'kind', kind, 'legacy_id', legacy_id, 'decision', decision,
                'reason', reason, 'authority_id', authority_id) order by seq)
            from public.legacy_resolutions), '[]')));
$$;

revoke all on all tables in schema public from anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.resolve_legacy_quarantine(text, jsonb) from public, anon;
grant execute on function public.resolve_legacy_quarantine(text, jsonb)
    to authenticated, service_role;
