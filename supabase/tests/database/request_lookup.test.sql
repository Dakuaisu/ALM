-- request_lookup resolves an unknown outcome against the stored command and terms, with no
-- effect; Owner only.
begin;
create extension if not exists pgtap with schema extensions;
select plan(6);
insert into public.products (id, item, sell_minor, stock) values (1, 'Synthetic product', 125, 5);
insert into public.product_lots (product_id, remaining) values (1, 5);
insert into public.customers (id, name) values ('c1', 'Synthetic Customer');
select set_config('request.jwt.claims', '{"app_metadata": {"role": "owner"}}', true);
create temp table terms as select '{"draft_id": "1", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 125}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30"}'::jsonb as t;

select is(public.request_lookup('r1', 'post_sale', (select t from terms)), '{"state": "absent"}'::jsonb,
    'nothing committed: absent');
select is((select count(*) from public.sale_requests), 0::bigint, 'the lookup records nothing');
select public.post_sale('r1', (select t from terms));
select is(public.request_lookup('r1', 'post_sale', (select t from terms)) -> 'outcome',
    '{"draft_id": "1", "invoice": 1, "quantity": 2, "total_minor": 250}'::jsonb,
    'committed: the original outcome');
select is(public.request_lookup('r1', 'set_sale_cancelled', '{"draft_id": "1", "cancel": true}') ->> 'state',
    'conflict', 'another intent under the request: conflict');
select set_config('request.jwt.claims', '{"app_metadata": {"role": "manager"}}', true);
select throws_ok($$ select public.request_lookup('r1', 'post_sale', '{}') $$, 'ALM06', null,
    'a Manager cannot look up');
select ok(has_function_privilege('authenticated', 'public.request_lookup(text, text, jsonb)', 'execute')
    and not has_function_privilege('anon', 'public.request_lookup(text, text, jsonb)', 'execute'),
    'authenticated may call it, anon may not');
select * from finish();
rollback;
