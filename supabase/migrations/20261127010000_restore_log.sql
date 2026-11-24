-- Local development schema, continued: the HTML's `restore` log entry. A device that restored a
-- backup file tells the authority, which logs "restore — Backup file restored" with the device
-- and account of the request, as the core's `Intent::RecordRestore`. Nothing else changes.
-- The app's Clear All stays inert, so no `clear` entry is ever recorded. Owner only (OPEN-004).

-- {} (no terms)
create function public.record_restore(p_request_id text, p_terms jsonb)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
    v_done jsonb;
begin
    perform alm_private.require_owner();
    v_done := alm_private.replay(p_request_id, 'record_restore', p_terms);
    if v_done is not null then
        return v_done;
    end if;
    if p_terms is distinct from '{}'::jsonb then
        perform alm_private.invalid('terms');
    end if;
    perform alm_private.bump();
    perform alm_private.log('restore', 'Backup file restored', null, '');
    return alm_private.record(p_request_id, 'record_restore', p_terms, '{"done": true}');
end;
$$;

revoke all on function public.record_restore(text, jsonb) from public, anon;
grant execute on function public.record_restore(text, jsonb) to authenticated, service_role;
