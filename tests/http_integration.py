"""Real HTTP/PostgreSQL/SMTP regression suite. Explicit disposable test DB only.
Selected scenarios originate in BSLT R1 access tests; lifecycle and owner-scope
cases extend that baseline. No mock authentication or email endpoint exists.
"""
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

DSN = os.environ.get('APP_DATABASE_URL', '')
db = urllib.parse.urlsplit(DSN)
assert os.environ.get('APP_MODE') == 'test'
assert db.hostname in ('127.0.0.1', '::1') and db.path.startswith('/starter_test_')
BASE = 'http://127.0.0.1:8088'
ORIGIN = 'http://127.0.0.1:5178'
MAIL = 'http://127.0.0.1:8025'
PASSWORD = 'integration test password only'
NEW_PASSWORD = 'replacement test password only'
checks = 0

def check(condition, label):
    global checks
    assert condition, label
    checks += 1

def sql(statement):
    environment = dict(os.environ, PGHOST=db.hostname, PGPORT=str(db.port or 5432), PGUSER=urllib.parse.unquote(db.username or ''), PGPASSWORD=urllib.parse.unquote(db.password or ''), PGDATABASE=db.path[1:])
    result = subprocess.run(['psql', '-X', '-v', 'ON_ERROR_STOP=1', '-At'], input=statement, text=True, capture_output=True, env=environment, timeout=10)
    if result.returncode: raise AssertionError('Test fixture SQL failed')
    return result.stdout.strip()

def call(method, path, body=None, cookie=None, origin=True):
    headers = {}
    if origin: headers.update({'Origin': ORIGIN, 'X-Starter-Client': 'web-v1'})
    if cookie: headers['Cookie'] = cookie
    data = None
    if body is not None: headers['Content-Type'] = 'application/json'; data = json.dumps(body).encode()
    req = urllib.request.Request(BASE + path, data=data, headers=headers, method=method)
    try: response = urllib.request.urlopen(req, timeout=10)
    except urllib.error.HTTPError as error: response = error
    with response:
        content = response.read(); status = response.status; response_headers = dict(response.headers)
    parsed = json.loads(content) if content else None
    return status, response_headers, parsed

def cookie_of(response):
    return next(value for key, value in response[1].items() if key.lower() == 'set-cookie').split(';')[0]

def credentials(email, remember=False, password=PASSWORD):
    return {'email': email, 'password': password, 'remember_me': remember}

def mail_token(email, purpose, exclude=None):
    deadline = time.monotonic() + 12
    while time.monotonic() < deadline:
        with urllib.request.urlopen(MAIL + '/api/v1/messages?limit=100', timeout=3) as response: listing = json.load(response)
        for message in listing.get('messages', []):
            if not any(to.get('Address') == email for to in message.get('To', [])): continue
            with urllib.request.urlopen(MAIL + '/api/v1/message/' + message['ID'], timeout=3) as response: value = json.load(response)
            found = re.search(r'#' + purpose + r'=([a-f0-9]{64})', value.get('Text', ''))
            if found and found.group(1) != exclude: return found.group(1)
        time.sleep(.15)
    raise AssertionError('Expected local SMTP message was not delivered')

def register(label, remember=False):
    email = f'{label}-{uuid.uuid4().hex[:8]}@example.test'
    result = call('POST', '/api/auth/register', credentials(email, remember))
    check(result[0] == 201, 'registration succeeds')
    return email, result[2]['user']['id'], cookie_of(result), result

def verify(email, cookie):
    token = mail_token(email, 'verify')
    check(call('POST','/api/auth/verify',{'token':token})[0] == 204,'verification succeeds')
    check(call('GET','/api/auth/session',cookie=cookie)[2]['user']['email_verified'],'verified status stored')
    return token

check(call('GET','/api/auth/session')[0] == 401,'anonymous session denied')
check(call('POST','/api/auth/register',credentials('blocked@example.test'),origin=False)[0] == 403,'missing origin denied')
check(call('POST','/api/auth/register',{'email':'bad','password':'short'})[0] == 422,'invalid input denied')
email, uid, first, registered = register('alice', True)
check('HttpOnly' in str(registered[1]) and 'SameSite=Strict' in str(registered[1]), 'cookie flags')
check('2592000' in str(registered[1]) and registered[2]['idle_timeout_seconds'] == 604800,'BSLT remembered policy')
check(call('GET','/api/notes',cookie=first)[0] == 403,'unverified notes denied')
check(call('POST','/api/auth/register',credentials(email.upper()))[0] == 409,'normalized duplicate denied')
check(sql(f"SELECT password_hash LIKE '$argon2id$%' FROM starter.users WHERE id='{uid}'") == 't','password hash is Argon2id')
check(sql(f"SELECT token_hash FROM starter.sessions WHERE user_id='{uid}'") == hashlib.sha256(first.split('=')[1].encode()).hexdigest(),'only token digest stored')
verification = verify(email,first)
check(call('POST','/api/auth/verify',{'token':verification})[0] == 400,'verification single use')
check(call('POST','/api/auth/reset-password',{'token':verification,'password':NEW_PASSWORD})[0] == 400,'token purpose bound')
created = call('POST','/api/notes',{'title':'First note','body':'private content'},cookie=first)
check(created[0] == 201,'note created'); note = created[2]
check(len(call('GET','/api/notes',cookie=first)[2]) == 1,'owner list')
other, oid, other_cookie, _ = register('bob')
verify(other,other_cookie)
check(call('GET','/api/notes',cookie=other_cookie)[2] == [],'cross-user list isolated')
check(call('PUT','/api/notes/'+note['id'],{'title':'attack','body':'','revision':1},cookie=other_cookie)[0] == 404,'cross-user update denied')
check(call('DELETE','/api/notes/'+note['id'],{'revision':1},cookie=other_cookie)[0] == 404,'cross-user delete denied')
updated = call('PUT','/api/notes/'+note['id'],{'title':'Updated','body':'new','revision':1},cookie=first)
check(updated[0] == 200 and updated[2]['revision'] == 2,'revision increments')
check(call('PUT','/api/notes/'+note['id'],{'title':'stale','body':'','revision':1},cookie=first)[0] == 409,'stale update denied')
check(call('DELETE','/api/notes/'+note['id'],{'revision':1},cookie=first)[0] == 409,'stale delete denied')
check(call('DELETE','/api/notes/'+note['id'],{'revision':2},cookie=first)[0] == 204,'delete succeeds')
check(call('PATCH','/api/account',{'display_name':' Alice ','theme':'dark'},cookie=first)[2]['display_name'] == 'Alice','profile persisted')
wrong = call('POST','/api/auth/login',credentials(email,password='wrong password'))
unknown = call('POST','/api/auth/login',credentials('missing@example.test'))
check(wrong[0] == 401 and wrong[2] == unknown[2], 'generic credential error')
logged = call('POST','/api/auth/login',credentials(email),cookie=first)
check(logged[0] == 200,'login succeeds'); second=cookie_of(logged)
check(first != second and call('GET','/api/auth/session',cookie=first)[0] == 401,'login rotates current session')
check(logged[2]['idle_timeout_seconds'] == 43200 and '43200' in str(logged[1]),'default BSLT session span')
check(call('POST','/api/auth/logout',cookie=second,origin=False)[0] == 403,'logout origin checked')
check(call('POST','/api/auth/logout',cookie=second)[0] == 204,'logout succeeds')
check(call('GET','/api/auth/session',cookie=second)[0] == 401,'logout revokes')
check(call('POST','/api/auth/logout')[0] == 204,'absent logout idempotent')
a = cookie_of(call('POST','/api/auth/login',credentials(other)))
b = cookie_of(call('POST','/api/auth/login',credentials(other)))
check(call('POST','/api/auth/logout-all',cookie=b)[0] == 204,'global revocation')
check(all(call('GET','/api/auth/session',cookie=c)[0] == 401 for c in (a,b)), 'all sessions revoked')
# Recovery is generic, single-use and never automatically logs the user in.
rmail, rid, rcookie, _ = register('recovery'); verify(rmail,rcookie)
check(call('POST','/api/auth/forgot-password',{'email':rmail})[0] == 202,'recovery accepted')
old = mail_token(rmail,'reset')
check(call('POST','/api/auth/forgot-password',{'email':rmail})[0] == 202,'replacement recovery accepted')
new = mail_token(rmail,'reset',old)
check(call('POST','/api/auth/reset-password',{'token':old,'password':NEW_PASSWORD})[0] == 400,'old recovery invalidated')
check(call('POST','/api/auth/forgot-password',{'email':'absent-recovery@example.test'})[0] == 202,'unknown recovery same status')
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
    results = list(pool.map(lambda _: call('POST','/api/auth/reset-password',{'token':new,'password':NEW_PASSWORD})[0], range(2)))
check(sorted(results) == [204,400], 'exactly one concurrent reset consumes token')
check(call('GET','/api/auth/session',cookie=rcookie)[0] == 401,'reset revokes existing session')
check(call('POST','/api/auth/login',credentials(rmail))[0] == 401,'old password rejected')
new_session = call('POST','/api/auth/login',credentials(rmail,password=NEW_PASSWORD))
check(new_session[0] == 200,'new password works')
check(call('POST','/api/auth/change-password',{'current_password':NEW_PASSWORD,'password':PASSWORD},cookie=cookie_of(new_session))[0] == 204,'authenticated password change')
check(call('GET','/api/auth/session',cookie=cookie_of(new_session))[0] == 401,'change revokes sessions')
# Expiry and disabled-user tests mutate only this deliberately disposable DB.
xmail, xid, xcookie, _ = register('expiry')
sql(f"UPDATE starter.sessions SET last_seen_at=now()-interval '13 hours' WHERE user_id='{xid}'")
check(call('GET','/api/auth/session',cookie=xcookie)[0] == 401,'idle expiry enforced')
x = cookie_of(call('POST','/api/auth/login',credentials(xmail)))
sql(f"UPDATE starter.sessions SET expires_at=now()-interval '1 second' WHERE user_id='{xid}'")
check(call('GET','/api/auth/session',cookie=x)[0] == 401,'absolute expiry enforced')
x = cookie_of(call('POST','/api/auth/login',credentials(xmail)))
sql(f"UPDATE starter.users SET disabled=true WHERE id='{xid}'")
check(call('GET','/api/auth/session',cookie=x)[0] == 401,'disabled session denied')
check(call('POST','/api/auth/login',credentials(xmail))[0] == 401,'disabled login denied')
# Checksummed migration history cannot be silently rewritten.
saved = sql('SELECT checksum FROM starter.schema_migrations WHERE version=1')
sql("UPDATE starter.schema_migrations SET checksum='tampered' WHERE version=1")
check(call('GET','/health/ready')[0] != 200,'migration drift fails readiness')
sql(f"UPDATE starter.schema_migrations SET checksum='{saved}' WHERE version=1")
check(call('GET','/health/ready')[0] == 200,'fixture restored')
print(f'HTTP/DB/SMTP checks passed: {checks}')
