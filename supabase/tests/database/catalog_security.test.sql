-- Catalog checks for a public repository (migration 20261202000000): no public table without
-- row level security, no table or sequence privilege for anon or authenticated, nothing anon
-- may call, every function authenticated may call SECURITY DEFINER with an empty search_path
-- and the role check as its first statement, and no dynamic SQL built from text without
-- format('%I'/'%L') or parameters. Each check is then shown to catch a planted violation.
begin;
create extension if not exists pgtap with schema extensions;
select plan(12);

create function pg_temp.rls_off() returns setof text language sql as $$
    select c.relnamespace::regnamespace::text || '.' || c.relname from pg_catalog.pg_class c
    where c.relnamespace = 'public'::regnamespace and c.relkind in ('r', 'p')
        and not c.relrowsecurity order by 1
$$;

create function pg_temp.api_grants() returns setof text language sql as $$
    select distinct c.relnamespace::regnamespace::text || '.' || c.relname || ' to ' || a.grantee::regrole::text
    from pg_catalog.pg_class c, pg_catalog.aclexplode(c.relacl) a
    where c.relnamespace in ('public'::regnamespace, 'alm_private'::regnamespace)
        and a.grantee in ('anon'::regrole, 'authenticated'::regrole) order by 1
$$;

create function pg_temp.app_functions() returns table (f regprocedure, src text, secdef boolean,
    config text[], lang name) language sql as $$
    select p.oid::regprocedure, p.prosrc, p.prosecdef, p.proconfig, l.lanname
    from pg_catalog.pg_proc p join pg_catalog.pg_language l on l.oid = p.prolang
    where p.pronamespace in ('public'::regnamespace, 'alm_private'::regnamespace)
        and not exists (select 1 from pg_catalog.pg_depend d
            where d.classid = 'pg_catalog.pg_proc'::regclass and d.objid = p.oid
                and d.deptype = 'e')
$$;

create function pg_temp.anon_callable() returns setof text language sql as $$
    select f::text from pg_temp.app_functions()
    where has_function_privilege('anon', f, 'execute') order by 1
$$;

create function pg_temp.not_definer() returns setof text language sql as $$
    select f::text from pg_temp.app_functions()
    where has_function_privilege('authenticated', f, 'execute')
        and not (secdef and 'search_path=""' = any(coalesce(config, '{}')))
    order by 1
$$;

create function pg_temp.check_not_first() returns setof text language sql as $$
    select f::text from pg_temp.app_functions()
    where has_function_privilege('authenticated', f, 'execute')
        and src !~* '^\s*begin\s+perform\s+alm_private\.require_(owner|member)\(\)\s*;'
    order by 1
$$;

-- An EXECUTE passes when its text is a format() whose template has no %s, or a single string
-- literal (parameters go through USING); anything concatenated or computed fails.
create function pg_temp.unsafe_dynamic_sql() returns setof text language sql as $$
    select f::text || ': ' || left(m[1], 60)
    from pg_temp.app_functions(), regexp_matches(src, '\mexecute\s+([^;]*)', 'gi') m
    where lang = 'plpgsql'
        and not (
            (m[1] ~* '^format\s*\(\s*''' and substring(m[1] from
                '^format\s*\(\s*''((?:[^'']|'''')*)''') !~ '%s')
            or (m[1] ~ '^''((?:[^'']|'''')*)''(\s|$)' and m[1] !~ '\|\|'))
    order by 1
$$;

select is_empty('select * from pg_temp.rls_off()', 'every public table has row level security on');
select is_empty('select * from pg_temp.api_grants()',
    'anon and authenticated hold no table, view or sequence privilege in public or alm_private');
select is_empty('select * from pg_temp.anon_callable()', 'anon may call no function');
select is_empty('select * from pg_temp.not_definer()',
    'every function authenticated may call is SECURITY DEFINER with search_path empty');
select is_empty('select * from pg_temp.check_not_first()',
    'every function authenticated may call runs the role check first');
select is_empty('select * from pg_temp.unsafe_dynamic_sql()',
    'no function builds dynamic SQL without format(%I/%L) or parameters');

create table public.zz_probe (id int);
grant select on public.zz_probe to anon;
create function public.zz_open(p text) returns text language plpgsql as $$
declare v text;
begin
    execute 'select ' || p into v;
    execute format('select %s', p) into v;
    return v;
end;
$$;
grant execute on function public.zz_open(text) to anon, authenticated;

select ok('public.zz_probe' in (select * from pg_temp.rls_off()), 'catches a table without RLS');
select ok('public.zz_probe to anon' in (select * from pg_temp.api_grants()), 'catches a grant to anon');
select ok('zz_open(text)' in (select * from pg_temp.anon_callable()), 'catches a function anon may call');
select ok('zz_open(text)' in (select * from pg_temp.not_definer()), 'catches a SECURITY INVOKER function');
select ok('zz_open(text)' in (select * from pg_temp.check_not_first()),
    'catches a function without the role check first');
select is((select count(*) from pg_temp.unsafe_dynamic_sql() u where u like 'zz_open(text):%'),
    2::bigint, 'catches concatenated and %s dynamic SQL');

select * from finish();
rollback;
