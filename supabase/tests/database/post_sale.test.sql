-- Mirrors phase2-local alm-core replays against post_sale:
--   confirmation::tests::successful_sale_and_lost_response_replay_have_one_effect
--   confirmation::tests::reused_request_with_different_review_conflicts
--   confirmation::tests::changed_draft_or_terms_rejects_stale_review
begin;
create extension if not exists pgtap with schema extensions;
select plan(17);
-- The functions require the Owner role claim (20261124000000_authority_commands.sql).
select set_config('request.jwt.claims', '{"app_metadata": {"role": "owner"}}', true);

insert into public.products (id, item, sell_minor, stock) values (1, 'Synthetic product', 125, 5);

-- Lost response: the first post commits once.
select is(
    public.post_sale('1', '{"draft_id": "1", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 125}]}'),
    '{"draft_id": "1", "invoice": 1, "quantity": 2, "total_minor": 250}'::jsonb,
    'first post returns invoice 1 for 2 units totalling 250');
select is((select stock from public.products where id = 1), 3, 'stock falls from 5 to 3');
select is((select sum(total_minor) from public.sales)::bigint, 250::bigint, 'ledger is 250');

-- The price changes, then the client retries the lost response with the original terms.
update public.products set sell_minor = 400 where id = 1;
select is(
    public.post_sale('1', '{"lines": [{"unit_price_minor": 125, "quantity": 2, "product_id": 1}], "draft_id": "1"}'),
    '{"draft_id": "1", "invoice": 1, "quantity": 2, "total_minor": 250}'::jsonb,
    'retry with identical terms (any key order) returns the original receipt');
select is((select stock from public.products where id = 1), 3, 'retry does not move stock');
select is((select count(*) from public.sale_requests), 1::bigint, 'one request is recorded');
select is((select count(*) from public.sales), 1::bigint, 'one sale is recorded');

-- The same draft under a new request ID is already posted.
select throws_ok(
    $$ select public.post_sale('2', '{"draft_id": "1", "lines": [{"product_id": 1, "quantity": 2, "unit_price_minor": 125}]}') $$,
    'ALM02', 'draft_already_posted', 'a new request for a posted draft is rejected');

-- Conflicting request: request 1 reused for a different draft.
update public.products set sell_minor = 125 where id = 1;
select throws_ok(
    $$ select public.post_sale('1', '{"draft_id": "2", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 125}]}') $$,
    'ALM01', 'request_conflict', 'reusing a request ID with different terms is rejected');
select throws_ok(
    $$ select public.post_sale('1', '{"draft_id": "1", "lines": [{"product_id": 1, "quantity": 3, "unit_price_minor": 125}]}') $$,
    'ALM01', 'request_conflict', 'reusing a request ID with a changed quantity is rejected');
select is((select stock from public.products where id = 1), 3, 'rejected requests do not move stock');

-- Stale terms and shortages leave everything unchanged.
select throws_ok(
    $$ select public.post_sale('3', '{"draft_id": "3", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 200}]}') $$,
    'ALM03', 'stale_review', 'a price other than the current sell price is stale');
select throws_ok(
    $$ select public.post_sale('4', '{"draft_id": "4", "lines": [{"product_id": 1, "quantity": 4, "unit_price_minor": 125}]}') $$,
    'ALM04', 'insufficient_stock', 'more than the stock on hand is rejected');
select is((select count(*) from public.sale_requests), 1::bigint, 'failed posts record no request');

-- A second sale takes the next invoice number.
select is(
    (public.post_sale('5', '{"draft_id": "5", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 125}]}') ->> 'invoice')::bigint,
    2::bigint, 'the next sale is invoice 2');

-- Deny-by-default: client roles cannot call the function or read the tables.
select is(has_function_privilege('anon', 'public.post_sale(text, jsonb)', 'execute'), false,
    'anon cannot post sales');
select is(has_table_privilege('authenticated', 'public.sales', 'select'), false,
    'authenticated cannot read sales');

select * from finish();
rollback;
