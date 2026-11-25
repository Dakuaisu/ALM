-- One transaction running many commands holds a bounded number of advisory locks (migration
-- 20261128040000): 20,000 commands in one transaction on a default postgres:17
-- (max_locks_per_transaction 64; one lock per command ran out of shared memory between 12,000
-- and 14,000) neither run out of shared memory nor hold more than the 1,024 request buckets.
begin;
create extension if not exists pgtap with schema extensions;
select plan(2);
select set_config('request.jwt.claims', '{"app_metadata": {"role": "owner"}}', true);
select lives_ok(
    $$select count(*) from (select public.set_reorder_default('locks-' || g,
        jsonb_build_object('level', 5)) from generate_series(1, 20000) g) x$$,
    '20,000 commands in one transaction');
select cmp_ok(
    (select count(*)::integer from pg_locks
        where pid = pg_backend_pid() and locktype = 'advisory'),
    '<=', 1024, 'at most 1,024 request locks held');
select * from finish();
rollback;
