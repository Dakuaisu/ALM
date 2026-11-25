-- Local development schema, continued: roles default to deny. A token's role comes from the
-- live GoTrue user it names (`auth.users`, not deleted, not banned; `app_metadata.role` there);
-- a token whose subject is no GoTrue user now has no role, so every command and pull is
-- refused. Its own `app_metadata.role` claim is read only while the setting
-- `alm.allow_claim_roles` is `on`, which only the plain-Postgres test bootstrap sets (for the
-- development tokens the tests mint with the server's secret); no migration sets it, and a
-- client reaching the database through PostgREST cannot set it.
-- Not the approved permission matrix (OPEN-004): unapproved operations stay denied.

create or replace function alm_private.app_role()
returns text
language plpgsql
stable
set search_path = ''
as $$
declare
    v_claims jsonb;
    v_sub text;
    v_role text;
    v_rows integer;
begin
    begin
        v_claims := auth.jwt();
    exception
        when undefined_function or invalid_schema_name then
            return '';
    end;
    v_sub := v_claims ->> 'sub';
    if to_regclass('auth.users') is not null
        and v_sub ~ '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
    then
        execute 'select raw_app_meta_data ->> ''role'' from auth.users
                 where id = $1::uuid and deleted_at is null
                     and (banned_until is null or banned_until <= now())'
            into v_role using v_sub;
        get diagnostics v_rows = row_count;
        if v_rows > 0 then
            return coalesce(v_role, '');
        end if;
        execute 'select count(*) from auth.users where id = $1::uuid' into v_rows using v_sub;
        if v_rows > 0 then
            return '';
        end if;
    end if;
    if coalesce(current_setting('alm.allow_claim_roles', true), '') <> 'on' then
        return '';
    end if;
    return coalesce(v_claims -> 'app_metadata' ->> 'role', '');
end;
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
