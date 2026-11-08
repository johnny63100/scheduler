-- Nora Schedule 클라우드 동기화용 Supabase 설정
-- Supabase 대시보드 → SQL Editor 에 통째로 붙여 넣고 Run 하세요. (한 번만)
--
-- 구조
--   nora_snapshot : PC 앱이 올리는 "지금 상태" (일정·할 일). 사용자당 한 줄.
--   nora_inbox    : 휴대폰에서 보낸 요청 (일정 추가, 할 일 추가, 할 일 체크).
--                   PC 앱이 읽어서 반영한 뒤 applied_at 을 채워요.
-- 모든 행은 로그인한 본인(auth.uid())만 읽고 쓸 수 있어요.

create table if not exists public.nora_snapshot (
  owner      uuid primary key default auth.uid() references auth.users on delete cascade,
  data       jsonb not null,
  updated_at timestamptz not null default now()
);

create table if not exists public.nora_inbox (
  id         bigint generated always as identity primary key,
  owner      uuid not null default auth.uid() references auth.users on delete cascade,
  kind       text not null check (kind in ('add_event', 'add_todo', 'set_todo_done')),
  payload    jsonb not null,
  created_at timestamptz not null default now(),
  applied_at timestamptz,
  error      text
);

create index if not exists nora_inbox_pending on public.nora_inbox (owner, id) where applied_at is null;

alter table public.nora_snapshot enable row level security;
alter table public.nora_inbox    enable row level security;

drop policy if exists "own snapshot" on public.nora_snapshot;
create policy "own snapshot" on public.nora_snapshot
  for all to authenticated
  using (owner = auth.uid()) with check (owner = auth.uid());

drop policy if exists "own inbox" on public.nora_inbox;
create policy "own inbox" on public.nora_inbox
  for all to authenticated
  using (owner = auth.uid()) with check (owner = auth.uid());

-- 휴대폰 화면이 PC 변경을 바로 받도록 실시간 알림 켜기 (이미 켜져 있으면 오류 없이 넘어가요)
do $$
begin
  alter publication supabase_realtime add table public.nora_snapshot;
exception when duplicate_object then null;
end $$;
do $$
begin
  alter publication supabase_realtime add table public.nora_inbox;
exception when duplicate_object then null;
end $$;
