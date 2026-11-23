-- Central nextGstNo: distinct numbers across devices, cancelled bills counted, deleted highest reused.
begin;
create extension if not exists pgtap with schema extensions;
select plan(8);

select is(public.allocate_gst_bill_number('device-a-1'), 1::bigint, 'first bill is number 1');
select is(public.allocate_gst_bill_number('device-b-1'), 2::bigint,
    'a second device gets the next number, not a duplicate');
select is(public.allocate_gst_bill_number('device-a-1'), 1::bigint, 'allocation is idempotent per bill');
update public.gst_bills set cancelled = true where id = 'device-b-1';
select is(public.allocate_gst_bill_number('device-a-2'), 3::bigint, 'cancelled bills keep their numbers');
delete from public.gst_bills where id = 'device-a-2';
select is(public.allocate_gst_bill_number('device-b-2'), 3::bigint,
    'deleting the highest-numbered bill makes its number next again, as in the HTML');
select is((select count(distinct number) from public.gst_bills), 3::bigint, 'no duplicates were issued');
select is(has_function_privilege('authenticated', 'public.allocate_gst_bill_number(text)', 'execute'),
    false, 'authenticated cannot allocate numbers yet');
select is(has_table_privilege('anon', 'public.gst_bills', 'select'), false, 'anon cannot read GST bills');

select * from finish();
rollback;
