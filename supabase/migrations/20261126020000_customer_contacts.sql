-- Local development schema, continued: customer contacts on the Customers screen. The
-- authority holds each directory customer's mobile and address and sends them with the
-- customer rows (Owner only, as every customer row is); the device lists and searches them as
-- the HTML does (`c.mobile`, `c.addr1, c.addr2`). Synthetic slice only (OPEN-002, OPEN-004).

alter table public.customers
    add column mobile text not null default '',
    add column addr1 text not null default '',
    add column addr2 text not null default '';

create function alm_private.customer_contacts(p_out jsonb)
returns jsonb
language sql
stable
set search_path = ''
as $$
    select case when jsonb_typeof(p_out #> '{rows,customers}') = 'array'
        then jsonb_set(p_out, '{rows,customers}', coalesce((select jsonb_agg(e
                || jsonb_build_object('mobile', coalesce(c.mobile, ''),
                    'addr1', coalesce(c.addr1, ''), 'addr2', coalesce(c.addr2, ''))
                order by n)
            from jsonb_array_elements(p_out #> '{rows,customers}') with ordinality t(e, n)
            left join public.customers c on c.id = e ->> 'id'), '[]'))
        else p_out end;
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
    end if;
    return v_out;
end;
$$;

revoke all on all functions in schema alm_private from public, anon, authenticated;
