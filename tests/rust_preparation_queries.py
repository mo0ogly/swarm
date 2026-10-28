"""Actual GLM-generated query refusals, with valid fixture and recovery witnesses."""
import urllib.parse

def verify_queries(server, preparation_id, check):
    """Verify preparation read query validation on the actual server.

    First witnesses the real fixture via an authorized good show GET (200),
    then exercises each invalid query (all must yield 400 with failure code
    preparation-query-invalid), confirms a missing safe id yields 404
    preparation-not-found, confirms an unauthorized good show is refused 403,
    and finally repeats the good show to prove identical results. Every
    request uses observe=False and GET with no body.
    """
    base = {'id': preparation_id}
    good_show = '/api/v1/preparations/show?' + urllib.parse.urlencode(base)
    first = server.request(good_show, authorized=True, status=200, observe=False)
    check('query-valid-show', first['id'] == preparation_id)
    invalid = [
        ('unknown-query', '/api/v1/preparations?' + urllib.parse.urlencode({'id': preparation_id})),
        ('notallowed', '/api/v1/preparations/methods?' + urllib.parse.urlencode({'id': preparation_id})),
        ('missing-id', '/api/v1/preparations/show'),
        ('duplicate-id', '/api/v1/preparations/show?' + urllib.parse.urlencode([('id', preparation_id), ('id', preparation_id)])),
        ('unsafe-id', '/api/v1/preparations/show?' + urllib.parse.urlencode({'id': '../etc/passwd'})),
        ('unknown-key', '/api/v1/preparations/show?' + urllib.parse.urlencode(dict(base, extra='1'))),
        ('missing-document', '/api/v1/preparations/history?' + urllib.parse.urlencode(base)),
        ('invalid-kind', '/api/v1/preparations/history?' + urllib.parse.urlencode(dict(base, document='other'))),
        ('duplicate-document', '/api/v1/preparations/history?' + urllib.parse.urlencode([('id', preparation_id), ('document', 'plan'), ('document', 'plan')])),
        ('before-0', '/api/v1/preparations/history?' + urllib.parse.urlencode(dict(base, document='plan', before='0'))),
        ('before-negative', '/api/v1/preparations/history?' + urllib.parse.urlencode(dict(base, document='plan', before='-1'))),
        ('before-noninteger', '/api/v1/preparations/history?' + urllib.parse.urlencode(dict(base, document='plan', before='x'))),
        ('before-int64-overflow', '/api/v1/preparations/history?' + urllib.parse.urlencode(dict(base, document='plan', before='9223372036854775808'))),
        ('duplicate-before', '/api/v1/preparations/history?' + urllib.parse.urlencode([('id', preparation_id), ('document', 'plan'), ('before', '1'), ('before', '1')])),
        ('before-empty', '/api/v1/preparations/history?' + urllib.parse.urlencode([('id', preparation_id), ('document', 'plan'), ('before', '')])),
    ]
    for name, path in invalid:
        parsed = server.request(path, authorized=True, status=400, observe=False)
        check('query-invalid-' + name, parsed.get('failure', {}).get('code') == 'preparation-query-invalid')
    missing = server.request(
        '/api/v1/preparations/show?' + urllib.parse.urlencode({'id': 'no-such-preparation'}),
        authorized=True, status=404, observe=False)
    check('missing-not-found', missing.get('failure', {}).get('code') == 'preparation-not-found')
    unauthorized = server.request(good_show, authorized=False, status=403, observe=False)
    check('query-unauthorized', unauthorized == 'web-access-refused')
    accepted = server.request(
        '/api/v1/preparations/history?' + urllib.parse.urlencode(dict(base, document='plan', before='1')),
        authorized=True, status=200, observe=False)
    check('before-1-empty', accepted == [])
    again = server.request(good_show, authorized=True, status=200, observe=False)
    check('show-repeat-stable', again == first)
