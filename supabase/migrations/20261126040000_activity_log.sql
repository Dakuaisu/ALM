-- Local development schema, continued: the activity log. The authority records the HTML's
-- `logAct` entries for the commands it commits — bill-save, bill-edit, bill-cancel,
-- bill-restore, payment, grn, adjust, prod-delete, crate-save, crate-edit, crate-delete,
-- gst-save, gst-edit, gst-cancel, gst-delete — with the time, the device (the `x-alm-device`
-- request header) and the account (the JWT `sub`). The parts of each message are stored; the
-- device formats them as the core does (`ActivityRow::message`). The rows travel in the Owner
-- book only (OPEN-004: who may read the log). Synthetic slice only (OPEN-002).

create table public.activity (
    id bigint generated always as identity primary key,
    at_ms bigint not null,
    device text not null,
    account text not null,
    kind text not null,
    subject text not null,
    amount_minor bigint,
    detail text not null default ''
);
alter table public.activity enable row level security;

create function alm_private.log(p_kind text, p_subject text, p_amount bigint, p_detail text)
returns void
language plpgsql
set search_path = ''
as $$
declare
    v_device text;
    v_account text;
begin
    begin
        v_device := current_setting('request.headers', true)::json ->> 'x-alm-device';
    exception when others then
        v_device := null;
    end;
    begin
        v_account := auth.jwt() ->> 'sub';
    exception when others then
        v_account := null;
    end;
    insert into public.activity (at_ms, device, account, kind, subject, amount_minor, detail)
        values ((extract(epoch from clock_timestamp()) * 1000)::bigint, coalesce(v_device, ''),
            coalesce(v_account, ''), p_kind, coalesce(p_subject, ''), p_amount,
            coalesce(p_detail, ''));
end;
$$;

-- `#no customer` of a bill.
create function alm_private.bill_subject(p_draft text)
returns text
language sql
stable
set search_path = ''
as $$
    select '#' || invoice_no || ' ' || customer_name from public.sales where draft_id = p_draft;
$$;

-- `fmtN` for whole numbers (en-IN grouping).
create function alm_private.fmt_n(p_value bigint)
returns text
language sql
immutable
set search_path = ''
as $$
    select to_char(p_value, 'FM9,99,99,99,99,99,99,999');
$$;

create function alm_private.on_command()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
    v_terms jsonb := new.terms;
    v_reason text;
begin
    case new.command
    when 'sell_crate' then
        perform alm_private.log('bill-save', alm_private.bill_subject(v_terms ->> 'draft_id'),
            (new.outcome ->> 'total_minor')::bigint, '');
    when 'edit_sale' then
        perform alm_private.log('bill-edit', alm_private.bill_subject(v_terms ->> 'draft_id'),
            (new.outcome ->> 'total_minor')::bigint, '');
    when 'record_receipt' then
        perform alm_private.log('payment',
            (select name from public.customers where id = v_terms ->> 'customer_id'),
            (v_terms ->> 'amount_minor')::bigint,
            case when (v_terms ->> 'online')::boolean then 'Online' else 'Cash' end);
    when 'receive_purchase_order' then
        perform alm_private.log('grn',
            'PO-' || lpad(((v_terms ->> 'po')::bigint + 1)::text, 3, '0') || ' '
                || (select supplier_name from public.purchase_orders
                    where id = (v_terms ->> 'po')::bigint),
            null, '');
    when 'save_crate' then
        perform alm_private.log(
            case when v_terms -> 'id' is null or jsonb_typeof(v_terms -> 'id') = 'null'
                then 'crate-save' else 'crate-edit' end,
            btrim(v_terms ->> 'name'), null, '');
    when 'count_stock' then
        if (v_terms ->> 'counted')::bigint <> (v_terms ->> 'on_hand')::bigint then
            v_reason := case when v_terms ->> 'reason' is null then null
                else case v_terms ->> 'reason'
                    when 'count' then 'Count correction'
                    when 'damage' then 'Damage / leakage'
                    when 'expired' then 'Expired'
                    when 'theft' then 'Theft / unknown'
                    when 'own-use' then 'Own use / sample'
                    else 'Other' end end;
            perform alm_private.log('adjust',
                (select item from public.products where id = (v_terms ->> 'product_id')::bigint),
                null,
                alm_private.fmt_n((v_terms ->> 'on_hand')::bigint) || ' → '
                    || alm_private.fmt_n((v_terms ->> 'counted')::bigint)
                    || coalesce(' (' || v_reason || ')', ''));
        end if;
    else
        null;
    end case;
    return null;
end;
$$;

create trigger activity_on_command after insert on public.command_requests
    for each row execute function alm_private.on_command();

create function alm_private.on_sale_request()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    perform alm_private.log('bill-save',
        alm_private.bill_subject((select draft_id from public.sales where id = new.sale_id)),
        (new.receipt ->> 'total_minor')::bigint, '');
    return null;
end;
$$;

create trigger activity_on_sale_request after insert on public.sale_requests
    for each row execute function alm_private.on_sale_request();

-- A bill cancelled or restored.
create function alm_private.on_sale_state()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    if new.cancelled and not old.cancelled then
        perform alm_private.log('bill-cancel', '#' || new.invoice_no || ' ' || new.customer_name,
            new.total_minor, '');
    elsif old.cancelled and not new.cancelled then
        perform alm_private.log('bill-restore', '#' || new.invoice_no || ' ' || new.customer_name,
            null, '');
    end if;
    return null;
end;
$$;

create trigger activity_on_sale_state after update of cancelled on public.sales
    for each row execute function alm_private.on_sale_state();

-- A standalone GST bill saved, edited or cancelled.
create function alm_private.on_gst_bill()
returns trigger
language plpgsql
set search_path = ''
as $$
declare
    v_subject text := 'GST ' || new.number || ' ' || coalesce(new.body ->> 'customer', '');
begin
    if new.cancelled and not old.cancelled then
        perform alm_private.log('gst-cancel', v_subject, null, '');
    elsif old.body is null and new.body is not null then
        perform alm_private.log('gst-save', v_subject, null, '');
    elsif old.body is not null and new.body is distinct from old.body
        and new.cancelled = old.cancelled then
        perform alm_private.log('gst-edit', v_subject, null, '');
    end if;
    return null;
end;
$$;

create trigger activity_on_gst_bill after update on public.gst_bills
    for each row execute function alm_private.on_gst_bill();

create function alm_private.on_gst_delete()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    if old.body is not null then
        perform alm_private.log('gst-delete',
            'GST ' || old.number || ' ' || coalesce(old.body ->> 'customer', ''), null, '');
    end if;
    return old;
end;
$$;

create trigger activity_on_gst_delete before delete on public.gst_bills
    for each row execute function alm_private.on_gst_delete();

create function alm_private.on_product_delete()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    perform alm_private.log('prod-delete', old.item, null, '');
    return old;
end;
$$;

create trigger activity_on_product_delete before delete on public.products
    for each row execute function alm_private.on_product_delete();

create function alm_private.on_crate_delete()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    perform alm_private.log('crate-delete', old.name, null, '');
    return old;
end;
$$;

create trigger activity_on_crate_delete before delete on public.crate_templates
    for each row execute function alm_private.on_crate_delete();

-- The book carries the log.
create function alm_private.activity_rows(p_out jsonb)
returns jsonb
language sql
stable
set search_path = ''
as $$
    select jsonb_set(p_out, '{rows,owner,book,activity}', coalesce((select jsonb_agg(
            jsonb_build_object('at_ms', at_ms, 'device', device, 'account', account,
                'kind', kind, 'subject', subject, 'amount', amount_minor, 'detail', detail)
            order by id) from public.activity), '[]'));
$$;

create or replace function public.pull_projection(p_checkpoint bigint)
returns jsonb
language plpgsql
security definer
stable
set search_path = ''
as $$
declare
    v_out jsonb := alm_private.pull_projection_body(p_checkpoint);
begin
    if v_out ->> 'role' = 'owner' and jsonb_typeof(v_out -> 'rows') = 'object' then
        v_out := jsonb_set(v_out, '{rows,owner,book}', alm_private.owner_book());
        v_out := alm_private.crate_rows(v_out);
        v_out := alm_private.customer_contacts(v_out);
        v_out := alm_private.activity_rows(v_out);
    end if;
    return v_out;
end;
$$;

revoke all on all tables in schema public from anon, authenticated;
revoke all on all functions in schema alm_private from public, anon, authenticated;
