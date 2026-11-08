# 휴대폰 동기화 설정

PC의 Nora Schedule이 일정과 할 일을 내 Supabase 프로젝트에 올리고,
휴대폰 웹앱(`mobile/index.html`)이 그걸 보여 줍니다. 휴대폰에서 **일정 추가 · 할 일 추가 · 할 일 체크**를 하면
PC 앱이 20초 안에 받아서 반영해요. PC가 꺼져 있으면 마지막으로 올린 내용을 보여 주고, 보낸 건 PC를 켤 때 반영돼요.

```
휴대폰 ──추가/체크──▶ nora_inbox ──▶ PC 앱 (nora.sqlite3에 저장)
휴대폰 ◀──보기────── nora_snapshot ◀── PC 앱 (변경 시 + 10분마다)
```

## 1. Supabase 프로젝트 (무료)

1. https://supabase.com 에서 새 프로젝트를 만들어요.
2. **SQL Editor**에 `mobile/supabase.sql` 내용을 붙여 넣고 Run.
3. **Authentication → Users → Add user → Create new user**: 이메일과 비밀번호 입력, *Auto Confirm User* 체크.
4. (권장) **Authentication → Sign In / Providers**에서 *Allow new users to sign up* 끄기. 다른 사람이 계정을 못 만들게 막아요.
5. **Project Settings → API Keys**에서 *Project URL*과 공개 키(`sb_publishable_…` 또는 `anon`)를 복사해 둬요.
   `service_role` / `secret` 키는 쓰지 않아요.

## 2. PC 앱 빌드 · 설치

1. 이 소스를 내 GitHub 저장소에 올려요.
2. **Actions → Build Windows installer → Run workflow**. 끝나면 실행 화면 아래 *Artifacts*에서 `nora-schedule-windows`를 받아요.
3. 받은 설치 파일을 실행해요. 기존 Nora Schedule 위에 덮어 설치되고 데이터는 그대로 남아요.
4. 앱 **설정 → 휴대폰 동기화**에 URL, 공개 키, 이메일, 비밀번호를 넣고 로그인.
   비밀번호는 로그인에만 쓰고 저장하지 않아요.

## 3. 휴대폰 웹앱

1. 저장소 **Settings → Pages → Source: GitHub Actions**로 바꿔요.
2. **Actions → Phone page → Run workflow**. 주소는 `https://<내아이디>.github.io/<저장소이름>/` 이에요.
3. 휴대폰으로 열어서 URL과 공개 키를 넣고 같은 계정으로 로그인해요.
4. 홈 화면에 추가해요 (iPhone: 공유 → 홈 화면에 추가 / Android: ⋮ → 홈 화면에 추가). 앱처럼 열려요.

## 알아 둘 점

- 휴대폰에서는 오늘 기준 **14일 전 ~ 120일 후** 일정, 안 끝난 할 일과 최근 3일 안에 끝낸 할 일이 보여요.
- 휴대폰에서 일정 수정·삭제는 아직 못 해요 (반복 일정 규칙은 PC에서만).
- Supabase 무료 프로젝트는 일주일 넘게 요청이 없으면 일시 정지돼요. PC 앱이 켜져 있으면 10분마다 올리니까 평소엔 괜찮아요.
- 앱의 업데이트 알림은 원본 저장소 릴리스를 가리켜요. 원본 버전을 설치하면 이 기능이 빠지니, 업데이트는 이 저장소에서 다시 빌드하세요.

## 화면 미러링

앱 수정 없이 **Chrome 원격 데스크톱**(무료)을 쓰는 게 가장 쉬워요.
PC에서 https://remotedesktop.google.com/access 에 들어가 원격 액세스를 켜고,
휴대폰에 *Chrome Remote Desktop* 앱을 설치하면 같은 구글 계정으로 PC 화면을 그대로 보고 조작할 수 있어요.
