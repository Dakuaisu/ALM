-- Shop-wide draft identity (migration 20261129000000): two devices of one account post their
-- first draft (`<account>:<device>-1`), each gets its own invoice; the earlier bare number is
-- still accepted; an ID outside 1-64 bytes of [A-Za-z0-9_:-] is refused as invalid terms
-- and writes nothing.
begin;
create extension if not exists pgtap with schema extensions;
select plan(12);
select set_config('request.jwt.claims', '{"app_metadata": {"role": "owner"}}', true);

insert into public.products (id, item, sell_minor, stock) values (1, 'Synthetic product', 125, 5);
insert into public.product_lots (product_id, remaining) values (1, 5);
insert into public.customers (id, name) values ('c1', 'Synthetic Customer');

select ok(alm_private.valid_draft_id('7'), 'a bare number is accepted');
select ok(alm_private.valid_draft_id('owner:dev-00000000000000aa-1'), 'origin and sequence');
select ok(alm_private.valid_draft_id(repeat('a', 22) || ':dev-0123456789abcdef-18446744073709551615'),
    'the longest origin with the largest sequence');
select ok(alm_private.valid_draft_id('p1'), 'a free-form ID of the allowed characters');
select ok(not alm_private.valid_draft_id(repeat('a', 65)), 'over 64 bytes is refused');
select ok(not alm_private.valid_draft_id(''), 'empty is refused');
select ok(not alm_private.valid_draft_id('a b-1'), 'a space is refused');
select ok(not alm_private.valid_draft_id('a/b-1'), 'a slash is refused');

select is(
    public.post_sale('owner:dev-00000000000000aa-1', '{"draft_id": "owner:dev-00000000000000aa-1", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 125}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30"}') ->> 'invoice',
    '1', 'device A''s first draft gets invoice 1');
select is(
    public.post_sale('owner:dev-00000000000000bb-1', '{"draft_id": "owner:dev-00000000000000bb-1", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 125}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30"}') ->> 'invoice',
    '2', 'device B''s first draft is not refused and gets invoice 2');
select throws_ok(
    $$ select public.post_sale('owner:dev-00000000000000cc-1', '{"draft_id": "bad draft", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 125}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30"}') $$,
    'ALM05', 'invalid_terms: draft id', 'a malformed draft ID is refused');
select is((select count(*) from public.sales), 2::bigint, 'the refused draft wrote nothing');
select * from finish();
rollback;
