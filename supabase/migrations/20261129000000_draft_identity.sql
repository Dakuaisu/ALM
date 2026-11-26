-- Local development schema, continued: shop-wide draft identity. Each device numbered its
-- drafts from 1 while `sales.draft_id` is unique for the whole shop, so a second device's
-- first bill was refused as already posted (found in the Android smoke). Devices now name a
-- draft `<account>:<device>-<n>` (the same origin their request IDs carry); a bare number is
-- the earlier per-device form, still accepted so queued commands and posted bills stay
-- valid. The invoice number is still issued only here, at posting. This migration bounds a
-- draft ID as request IDs are bounded (1-64 bytes of letters, digits, `_`, `:`, `-`, which
-- every device form satisfies); the exact `<origin>-<n>` form is the core's
-- `DraftId::parse`. Anything else is refused as invalid terms before a row is written.

create function alm_private.valid_draft_id(p_draft text)
returns boolean
language sql
immutable
set search_path = ''
as $$
    select p_draft is not null
        and pg_catalog.octet_length(p_draft) between 1 and 64
        and p_draft operator(pg_catalog.~) '^[A-Za-z0-9_:-]+$';
$$;

create function alm_private.check_draft_id()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
    if not alm_private.valid_draft_id(new.draft_id) then
        raise exception 'invalid_terms: draft id' using errcode = 'ALM05';
    end if;
    return new;
end;
$$;

create trigger sales_draft_id_shape
    before insert or update of draft_id on public.sales
    for each row execute function alm_private.check_draft_id();

revoke all on function alm_private.valid_draft_id(text) from public, anon, authenticated;
revoke all on function alm_private.check_draft_id() from public, anon, authenticated;
