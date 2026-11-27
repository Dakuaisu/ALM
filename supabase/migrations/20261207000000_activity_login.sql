-- Local development schema, continued: the activity log names the account by its login
-- identity (DEC-034): the JWT's `email`, or its `sub` for a token without one (the server
-- checks' minted tokens). Owner-only to read, as before. The body is 20261126040000's `log`
-- with that one change.

create or replace function alm_private.log(p_kind text, p_subject text, p_amount bigint, p_detail text)
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
        v_account := coalesce(nullif(auth.jwt() ->> 'email', ''), auth.jwt() ->> 'sub');
    exception when others then
        v_account := null;
    end;
    insert into public.activity (at_ms, device, account, kind, subject, amount_minor, detail)
        values ((extract(epoch from clock_timestamp()) * 1000)::bigint, coalesce(v_device, ''),
            coalesce(v_account, ''), p_kind, coalesce(p_subject, ''), p_amount,
            coalesce(p_detail, ''));
end;
$$;

revoke all on function alm_private.log(text, text, bigint, text) from public, anon, authenticated;
