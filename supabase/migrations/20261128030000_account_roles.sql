-- Local development schema, continued: real accounts (R-5). Each person signs in as a GoTrue
-- user (email and password); the authority's role for the user is `app_metadata.role`
-- ('owner' or 'manager'), which only an admin or service call can set (GoTrue's admin API,
-- e.g. `PUT /admin/users/<id>` with `{"app_metadata": {"role": ...}}`). `user_metadata`, which
-- a signed-in user may edit (`PUT /user`), is never read. When the token's subject is a GoTrue
-- user, the role is read from `auth.users` at call time, so a role an admin changed, removed or
-- a ban takes effect at once instead of when the token expires; another token (a development
-- token minted with the server's own secret, no GoTrue user) keeps its `app_metadata` claim.
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
        -- A deleted or banned GoTrue user has no role.
        execute 'select count(*) from auth.users where id = $1::uuid' into v_rows using v_sub;
        if v_rows > 0 then
            return '';
        end if;
    end if;
    return coalesce(v_claims -> 'app_metadata' ->> 'role', '');
end;
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
