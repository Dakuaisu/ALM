-- Incremental pulls (migration 20261129010000): the first pull is full and names its rows; a
-- pull from that checkpoint after one change carries only the changed lists and rows; nothing
-- new gives no rows; a checkpoint older than the kept history, or another role's checkpoint,
-- gets the full rows again; a Manager's changes hold no Owner rows.
begin;
create extension if not exists pgtap with schema extensions;
select plan(12);
select set_config('request.jwt.claims', '{"app_metadata": {"role": "owner"}}', true);
insert into public.products (id, item, sell_minor, stock, provisional_minor)
    values (1, 'Synthetic product', 125, 5, 125), (2, 'Synthetic soda', 300, 5, 300);
insert into public.product_lots (product_id, remaining) values (1, 5), (2, 5);
insert into public.customers (id, name) values ('c1', 'Synthetic Customer');

create temp table pulls (tag text primary key, body jsonb);
insert into pulls select 'first', public.pull_changes(null, 'owner');
select is((select body ->> 'full' from pulls where tag = 'first'), 'true', 'the first pull is full');
select ok((select body -> 'row_hashes' ? 'products' from pulls where tag = 'first'),
    'a full pull names its rows');

select public.set_reorder_default('incr-1', '{"level": 6}');
select public.post_sale('incr-2', '{"draft_id": "owner:dev-00000000000000aa-1", "lines": [{"product_id": 1, "quantity": 1, "unit_price_minor": 125}], "customer_id": "c1", "customer": "Synthetic Customer", "date": "2025-04-30"}');
insert into pulls select 'second', public.pull_changes(
    (select (body ->> 'checkpoint')::bigint from pulls where tag = 'first'), 'owner');
select is((select body ->> 'full' from pulls where tag = 'second'), 'false', 'changes only');
select is((select body -> 'delta' -> 'rows' -> 'catalog' from pulls where tag = 'second'),
    '{"$same": true}'::jsonb, 'an unchanged list is not resent');
select is((select jsonb_array_length(body -> 'delta' -> 'rows' -> 'sales' -> '$rows')
    from pulls where tag = 'second'), 1, 'the sales list names its one row');
select ok((select (select count(*) from jsonb_object_keys(body -> 'delta' -> 'added')) between 1 and 20
    from pulls where tag = 'second'), 'only new rows are carried');
select ok((select body -> 'delta' -> 'removed' ? 'stock' from pulls where tag = 'second'),
    'the stock row that changed is a tombstone');

insert into pulls select 'third', public.pull_changes(
    (select (body ->> 'checkpoint')::bigint from pulls where tag = 'second'), 'owner');
select ok((select body -> 'rows' = 'null'::jsonb and body -> 'delta' is null
    from pulls where tag = 'third'), 'nothing new: no rows');

select is((public.pull_changes((select (body ->> 'checkpoint')::bigint from pulls
    where tag = 'first'), 'manager')) ->> 'full', 'true', 'another role''s checkpoint: full');

update alm_private.projection_log set keep = 0 where role = 'owner';
select public.set_reorder_default('incr-3', '{"level": 7}');
select public.pull_changes(null, 'owner');
select public.set_reorder_default('incr-4', '{"level": 8}');
select is((public.pull_changes((select (body ->> 'checkpoint')::bigint from pulls
    where tag = 'second'), 'owner')) ->> 'full', 'true', 'older than the kept history: full');

select set_config('request.jwt.claims', '{"app_metadata": {"role": "manager"}}', true);
insert into pulls select 'm1', public.pull_changes(null, 'manager');
select set_config('request.jwt.claims', '{"app_metadata": {"role": "owner"}}', true);
update public.products set provisional_minor = 130 where id = 1;
update public.authority_state set revision = revision + 1 where id = 1;
select set_config('request.jwt.claims', '{"app_metadata": {"role": "manager"}}', true);
insert into pulls select 'm2', public.pull_changes(
    (select (body ->> 'checkpoint')::bigint from pulls where tag = 'm1'), 'manager');
select is((select body ->> 'full' from pulls where tag = 'm2'), 'false', 'Manager changes');
select ok((select position('owner' in (body -> 'delta')::text) = 0
    and position('cost' in (body -> 'delta')::text) = 0 from pulls where tag = 'm2'),
    'a Manager''s changes hold no Owner rows or costs');
select * from finish();
rollback;
