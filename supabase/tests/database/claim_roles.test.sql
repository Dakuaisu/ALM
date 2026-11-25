-- Roles default to deny (migration 20261128060000): a token whose subject is no GoTrue user
-- gets its app_metadata role only while alm.allow_claim_roles is on (test bootstrap only).
begin;
create extension if not exists pgtap with schema extensions;
select plan(7);
select set_config('request.jwt.claims',
    '{"sub": "claim-only", "app_metadata": {"role": "owner"}}', true);
select set_config('alm.allow_claim_roles', 'off', true);
select is(alm_private.app_role(), '', 'a claim-only owner token has no role with the setting off');
select throws_ok($$select public.set_reorder_default('claim-off', '{"level": 5}')$$,
    'ALM06', 'denied', 'its Owner command is denied');
select set_config('alm.allow_claim_roles', '', true);
select is(alm_private.app_role(), '', 'nor with the setting empty');
select set_config('request.jwt.claims',
    '{"sub": "00000000-0000-4000-8000-000000000000", "app_metadata": {"role": "owner"}}', true);
select set_config('alm.allow_claim_roles', 'off', true);
select is(alm_private.app_role(), '', 'nor when the subject looks like a user ID that does not exist');
select set_config('request.jwt.claims', '{"app_metadata": {"role": "manager"}}', true);
select is(alm_private.app_role(), '', 'a manager claim without a subject has no role either');
select set_config('alm.allow_claim_roles', 'on', true);
select is(alm_private.app_role(), 'manager', 'with the setting on the claim is read');
select set_config('request.jwt.claims',
    '{"sub": "claim-only", "app_metadata": {"role": "owner"}}', true);
select lives_ok($$select public.set_reorder_default('claim-on', '{"level": 5}')$$,
    'and an Owner command runs');
select * from finish();
rollback;
