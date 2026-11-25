-- Plain Postgres only (disposable test containers): stands in for the Supabase platform objects
-- the migrations expect. auth.jwt() reads request.jwt.claims exactly as Supabase's does.
create role anon nologin;
create role authenticated nologin;
create role service_role nologin;
create schema auth;
create function auth.jwt() returns jsonb
language sql
stable
as $$
    select coalesce(
        nullif(current_setting('request.jwt.claim', true), ''),
        nullif(current_setting('request.jwt.claims', true), ''))::jsonb
$$;
grant usage on schema auth to anon, authenticated, service_role;
grant execute on function auth.jwt() to anon, authenticated, service_role;
grant usage on schema public to anon, authenticated, service_role;
create schema extensions;
create extension pgtap with schema extensions;
alter database postgres set search_path = "$user", public, extensions;
-- The tests' development tokens (minted with the server's secret, no GoTrue user) carry their
-- role as a claim; migration 20261128060000 reads such a claim only with this setting on.
-- Test databases only: no migration sets it, so a hosted project refuses claim-only tokens.
alter database postgres set alm.allow_claim_roles = on;
