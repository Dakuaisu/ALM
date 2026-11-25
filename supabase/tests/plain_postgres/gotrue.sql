-- Plain Postgres only (disposable test containers), after bootstrap.sql and the migrations:
-- lets a GoTrue container run its own migrations in the `auth` schema, as the Supabase image
-- arranges for its auth service. The role's password is set by the test script from its
-- environment, never committed.
do $$
begin
    if not exists (select 1 from pg_roles where rolname = 'supabase_auth_admin') then
        create role supabase_auth_admin login createrole;
    end if;
end;
$$;
alter schema auth owner to supabase_auth_admin;
alter function auth.jwt() owner to supabase_auth_admin;
grant create on database postgres to supabase_auth_admin;
