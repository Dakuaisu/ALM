-- Local development schema, continued: incremental pulls. A device that holds a role's rows
-- at a checkpoint gets only what changed since it instead of the whole book on every pull.
-- Every array of rows at the top of the pulled rows, of `owner` and of `owner.book` is a
-- tracked list whose rows are named by a hash of their content (64 bits of the md5 of the
-- row's jsonb text).
-- Per role the authority records when each (list, row) was first seen and when it went, and
-- when each list last changed, as of the revision a pull saw; history older than `keep`
-- revisions is dropped. `pull_changes` answers a checkpoint with
--   {"full": false, "delta": {"base", "rows", "added", "removed"}}
-- where `rows` is the rows object with every tracked list replaced by {"$same": true}
-- (unchanged since the checkpoint) or {"$rows": [hash, ...]} (its rows in order), `added` the
-- rows first seen after the checkpoint by hash, and `removed` the rows that went since, by
-- list (tombstones); or with the full rows ({"full": true, "rows": ...}) when the device has no
-- checkpoint, its checkpoint is older than the retained history, or it holds another role's
-- rows; a full answer names every row (`row_hashes`: list path -> hashes in order) so the
-- device can take later changes. The same rule runs in the core's LocalAuthority
-- (alm_sync::delta). pull_projection is unchanged.

create table alm_private.projection_log (
    role text primary key,
    floor bigint not null,
    synced bigint,
    keep bigint not null default 10000 check (keep >= 0)
);

create table alm_private.projection_rows (
    role text not null,
    path text not null,
    hash text not null,
    first_seen bigint not null,
    gone_at bigint,
    primary key (role, path, hash)
);
create index projection_rows_gone on alm_private.projection_rows (role, gone_at)
    where gone_at is not null;

create table alm_private.projection_lists (
    role text not null,
    path text not null,
    list_hash text not null,
    changed_at bigint not null,
    primary key (role, path)
);

-- The tracked lists of a rows object: (path, rows).
create function alm_private.row_lists(p_rows jsonb)
returns table (path text, items jsonb)
language sql
immutable
set search_path = ''
as $$
    select e.key, e.value from pg_catalog.jsonb_each(p_rows) e
        where pg_catalog.jsonb_typeof(e.value) = 'array'
    union all
    select 'owner.' || e.key, e.value
        from pg_catalog.jsonb_each(case when pg_catalog.jsonb_typeof(p_rows -> 'owner') = 'object'
            then p_rows -> 'owner' else '{}'::jsonb end) e
        where pg_catalog.jsonb_typeof(e.value) = 'array'
    union all
    select 'owner.book.' || e.key, e.value
        from pg_catalog.jsonb_each(
            case when pg_catalog.jsonb_typeof(p_rows -> 'owner' -> 'book') = 'object'
            then p_rows -> 'owner' -> 'book' else '{}'::jsonb end) e
        where pg_catalog.jsonb_typeof(e.value) = 'array';
$$;

-- Records `p_rows` as `p_role`'s rows at `p_rev` (the rows are in pg_temp.projection_now).
create function alm_private.sync_projection(p_role text, p_rev bigint, p_rows jsonb)
returns void
language plpgsql
set search_path = ''
as $$
declare
    v_log alm_private.projection_log%rowtype;
    v_floor bigint;
begin
    select * into v_log from alm_private.projection_log where role = p_role for update;
    if not found then
        insert into alm_private.projection_log (role, floor) values (p_role, p_rev)
            returning * into v_log;
    end if;
    if v_log.synced = p_rev then
        return;
    end if;
    insert into alm_private.projection_lists as l (role, path, list_hash, changed_at)
        select p_role, r.path, pg_catalog.md5(coalesce((select pg_catalog.string_agg(n.hash, ','
                order by n.ord) from pg_temp.projection_now n where n.path = r.path), '')), p_rev
        from alm_private.row_lists(p_rows) r
        on conflict (role, path) do update set list_hash = excluded.list_hash,
            changed_at = excluded.changed_at
        where l.list_hash <> excluded.list_hash;
    delete from alm_private.projection_lists l
        where l.role = p_role
            and not exists (select 1 from alm_private.row_lists(p_rows) r where r.path = l.path);
    update alm_private.projection_rows r set gone_at = p_rev
        where r.role = p_role and r.gone_at is null
            and not exists (select 1 from pg_temp.projection_now n
                where n.path = r.path and n.hash = r.hash);
    insert into alm_private.projection_rows as r (role, path, hash, first_seen)
        select distinct p_role, n.path, n.hash, p_rev from pg_temp.projection_now n
        on conflict (role, path, hash) do update set first_seen = p_rev, gone_at = null
        where r.gone_at is not null;
    v_floor := greatest(v_log.floor, p_rev - v_log.keep);
    delete from alm_private.projection_rows r
        where r.role = p_role and r.gone_at is not null and r.gone_at < v_floor;
    update alm_private.projection_log set floor = v_floor, synced = p_rev where role = p_role;
end;
$$;

-- The changes from `p_base` to `p_rows` (synced, in pg_temp.projection_now), or null when
-- `p_base` is outside the retained history.
create function alm_private.projection_delta(p_role text, p_base bigint, p_rows jsonb)
returns jsonb
language plpgsql
set search_path = ''
as $$
declare
    v_log alm_private.projection_log%rowtype;
    v_skeleton jsonb := p_rows;
    v_list record;
    v_changed text[] := '{}';
begin
    select * into v_log from alm_private.projection_log where role = p_role;
    if not found or p_base < v_log.floor or v_log.synced is null or p_base > v_log.synced then
        return null;
    end if;
    for v_list in
        select r.path, l.changed_at from alm_private.row_lists(p_rows) r
            left join alm_private.projection_lists l on l.role = p_role and l.path = r.path
    loop
        if v_list.changed_at is not null and v_list.changed_at <= p_base then
            v_skeleton := pg_catalog.jsonb_set(v_skeleton,
                pg_catalog.string_to_array(v_list.path, '.'), '{"$same": true}');
        else
            v_changed := v_changed || v_list.path;
            v_skeleton := pg_catalog.jsonb_set(v_skeleton,
                pg_catalog.string_to_array(v_list.path, '.'),
                pg_catalog.jsonb_build_object('$rows', coalesce((select pg_catalog.jsonb_agg(
                    n.hash order by n.ord) from pg_temp.projection_now n
                    where n.path = v_list.path), '[]'::jsonb)));
        end if;
    end loop;
    return pg_catalog.jsonb_build_object(
        'base', p_base,
        'rows', v_skeleton,
        'added', coalesce((select pg_catalog.jsonb_object_agg(a.hash, a.body) from (
            select distinct on (n.hash) n.hash, n.body from pg_temp.projection_now n
                join alm_private.projection_rows r
                    on r.role = p_role and r.path = n.path and r.hash = n.hash
                where n.path = any (v_changed) and r.first_seen > p_base
                order by n.hash) a), '{}'::jsonb),
        'removed', coalesce((select pg_catalog.jsonb_object_agg(g.path, g.hashes) from (
            select r.path, pg_catalog.jsonb_agg(r.hash order by r.hash) hashes
                from alm_private.projection_rows r
                where r.role = p_role and r.gone_at > p_base and r.first_seen <= p_base
                group by r.path) g), '{}'::jsonb));
end;
$$;

-- What a device holding `p_role`'s rows at `p_checkpoint` needs: nothing new, the changes
-- since, or (no checkpoint, older than the history, another role) the full rows.
create function public.pull_changes(p_checkpoint bigint, p_role text default null)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_role text := alm_private.app_role();
    v_rev bigint := (select revision from public.authority_state where id = 1);
    v_full jsonb;
    v_delta jsonb;
begin
    if p_checkpoint is not null and p_checkpoint = v_rev
        and (p_role is null or p_role = v_role) then
        return public.pull_projection(p_checkpoint);
    end if;
    v_full := public.pull_projection(null);
    v_role := v_full ->> 'role';
    v_rev := (v_full ->> 'checkpoint')::bigint;
    perform alm_private.xact_lock(3, 'projection:' || v_role);
    create temp table if not exists projection_now (
        path text not null, ord bigint not null, hash text not null, body jsonb not null
    ) on commit drop;
    truncate pg_temp.projection_now;
    insert into pg_temp.projection_now
        select r.path, e.ord, pg_catalog.left(pg_catalog.md5(e.value::text), 16), e.value
        from alm_private.row_lists(v_full -> 'rows') r
            cross join lateral pg_catalog.jsonb_array_elements(r.items) with ordinality e(value, ord);
    create index if not exists projection_now_key on pg_temp.projection_now (path, hash);
    analyze pg_temp.projection_now;
    perform alm_private.sync_projection(v_role, v_rev, v_full -> 'rows');
    if p_checkpoint is not null and (p_role is null or p_role = v_role) then
        v_delta := alm_private.projection_delta(v_role, p_checkpoint, v_full -> 'rows');
    end if;
    if v_delta is null then
        return v_full || pg_catalog.jsonb_build_object('full', true, 'row_hashes',
            coalesce((select pg_catalog.jsonb_object_agg(h.path, h.names) from (
                select n.path, pg_catalog.jsonb_agg(n.hash order by n.ord) names
                    from pg_temp.projection_now n group by n.path) h), '{}'::jsonb));
    end if;
    return pg_catalog.jsonb_build_object('role', v_role, 'checkpoint', v_rev,
        'as_of_ms', v_full -> 'as_of_ms', 'full', false, 'rows', null, 'delta', v_delta);
end;
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
revoke all on function public.pull_changes(bigint, text) from public, anon;
grant execute on function public.pull_changes(bigint, text) to authenticated, service_role;
