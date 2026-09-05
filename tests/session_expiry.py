"""Expiry after a real user-row lock wait. Only an isolated test database is allowed."""
import concurrent.futures
import json
import os
import re
import select
import subprocess
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

url = urllib.parse.urlsplit(os.environ.get('APP_DATABASE_URL', ''))
assert os.environ.get('APP_MODE') == 'test'
assert url.hostname in ('127.0.0.1', '::1') and re.fullmatch(r'/starter_test_[A-Za-z0-9_]+', url.path)
environment = dict(os.environ, PGHOST=url.hostname, PGPORT=str(url.port or 5432), PGUSER=urllib.parse.unquote(url.username or ''), PGPASSWORD=urllib.parse.unquote(url.password or ''), PGDATABASE=url.path[1:])

def sql(statement):
    value = subprocess.run(['psql', '-X', '-qAt', '-v', 'ON_ERROR_STOP=1'], input=statement, text=True, capture_output=True, env=environment, timeout=5)
    if value.returncode: raise AssertionError('Test-only SQL failed')
    return value.stdout.strip()

def call(path, body=None, cookie=None):
    headers = {'Origin':'http://127.0.0.1:5178','X-Starter-Client':'web-v1'}
    if cookie: headers['Cookie'] = cookie
    if body is not None: headers['Content-Type'] = 'application/json'
    req = urllib.request.Request('http://127.0.0.1:8088'+path, data=None if body is None else json.dumps(body).encode(), headers=headers)
    try: response = urllib.request.urlopen(req, timeout=8)
    except urllib.error.HTTPError as error: response = error
    with response: return response.status, response.headers.get('Set-Cookie','').split(';')[0], json.loads(response.read())

email = 'lock-'+uuid.uuid4().hex+'@example.test'
credentials = {'email':email,'password':'test-only lock expiry password'}
status, _, value = call('/api/auth/register', credentials)
assert status == 201, 'Test-only registration failed'
uid = value['user']['id']; assert re.fullmatch(r'[a-f0-9-]{36}',uid)
try:
    for column in ('expires_at','last_seen_at'):
        status, cookie, _ = call('/api/auth/login', credentials); assert status == 200
        locker = subprocess.Popen(['psql','-X','-qAt','-v','ON_ERROR_STOP=1'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=environment)
        try:
            locker.stdin.write(f"BEGIN; SELECT id FROM starter.users WHERE id='{uid}' FOR UPDATE;\n\\echo LOCK_READY\n".encode()); locker.stdin.flush()
            output=b''; deadline=time.monotonic()+3
            while b'LOCK_READY' not in output:
                remaining=deadline-time.monotonic(); assert remaining>0, 'Lock holder did not start'
                ready,_,_=select.select([locker.stdout],[],[],remaining); assert ready, 'Lock holder timed out'
                chunk=os.read(locker.stdout.fileno(),4096); assert chunk, 'Lock holder exited'; output+=chunk
            with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
                pending=pool.submit(call,'/api/auth/session',None,cookie)
                deadline=time.monotonic()+2
                while sql("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND position('FOR UPDATE OF u' in query)>0") == '0':
                    assert time.monotonic()<deadline, 'Request never waited for the user lock'; time.sleep(.02)
                expression = "clock_timestamp()-interval '1 millisecond'" if column == 'expires_at' else "clock_timestamp()-idle_seconds*interval '1 second'-interval '1 millisecond'"
                sql(f"UPDATE starter.sessions SET {column}={expression} WHERE user_id='{uid}'")
                locker.stdin.write(b'COMMIT;\n\\q\n'); locker.stdin.flush()
                assert pending.result(timeout=5)[0] == 401, 'A session expired during the lock wait was accepted'
        finally:
            if locker.poll() is None: locker.terminate()
            locker.wait(timeout=3)
finally:
    sql(f"DELETE FROM starter.users WHERE id='{uid}'")
print('Lock-wait session expiry checks passed: 2')
