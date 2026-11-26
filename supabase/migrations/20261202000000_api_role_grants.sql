-- Local development schema, continued: API privileges for a public repository.
-- 1. The API roles hold no table or sequence privilege in public, and nothing the migration
--    role creates there later is granted to them by default. A hosted project grants every new
--    public table, sequence and function to anon and authenticated; legacy_resolutions_seq_seq
--    (20261128020000) kept that grant.
-- 2. Every function authenticated may call starts with the role check. Several read tables or
--    cast their arguments in DECLARE before their body's check; each moves to
--    alm_private.<name>_api unchanged, and public.<name> becomes a SECURITY DEFINER entry point
--    whose first statement is the check (Owner, or Owner or Manager for the two pulls).

revoke all on all tables in schema public from anon, authenticated;
revoke all on all sequences in schema public from anon, authenticated;
revoke all on all functions in schema public from public, anon;

alter default privileges in schema public revoke all on tables from anon, authenticated;
alter default privileges in schema public revoke all on sequences from anon, authenticated;
alter default privileges in schema public revoke all on functions from public, anon, authenticated;
alter default privileges in schema alm_private revoke all on functions from public;

create function alm_private.require_member()
returns void
language plpgsql
stable
set search_path = ''
as $$
begin
    if alm_private.app_role() not in ('owner', 'manager') then
        raise exception 'denied' using errcode = 'ALM06';
    end if;
end;
$$;

do $$
declare
    f record;
    v_args text;
    v_check text;
begin
    for f in
        select p.oid, p.proname, pg_catalog.pg_get_function_arguments(p.oid) as args,
            pg_catalog.pg_get_function_identity_arguments(p.oid) as ident,
            pg_catalog.pg_get_function_result(p.oid) as result, p.provolatile, p.proargnames
        from pg_catalog.pg_proc p
        where p.pronamespace = 'public'::regnamespace
            and pg_catalog.has_function_privilege('authenticated', p.oid, 'execute')
        order by p.proname
    loop
        select pg_catalog.string_agg(pg_catalog.quote_ident(a), ', ' order by i)
            into v_args from pg_catalog.unnest(f.proargnames) with ordinality u(a, i);
        v_check := case when f.proname in ('pull_projection', 'pull_changes')
            then 'require_member' else 'require_owner' end;
        execute format('alter function public.%I(%s) set schema alm_private', f.proname, f.ident);
        execute format('alter function alm_private.%I(%s) rename to %I',
            f.proname, f.ident, f.proname || '_api');
        execute format('revoke all on function alm_private.%I(%s) from public, anon, authenticated',
            f.proname || '_api', f.ident);
        execute format(
            'create function public.%I(%s) returns %s language plpgsql security definer %s '
            'set search_path = '''' as $body$ begin perform alm_private.%I(); '
            'return alm_private.%I(%s); end; $body$',
            f.proname, f.args, f.result,
            case f.provolatile when 's' then 'stable' else 'volatile' end,
            v_check, f.proname || '_api', v_args);
        execute format('revoke all on function public.%I(%s) from public, anon', f.proname, f.ident);
        execute format('grant execute on function public.%I(%s) to authenticated, service_role',
            f.proname, f.ident);
    end loop;
end;
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
