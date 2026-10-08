"""Recette HTTP réelle de Casa Pizza avec serveur et base jetables."""
import http.cookiejar
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.error
import urllib.request

ROOT = Path(os.environ.get('PIZZA_SOURCE', Path(__file__).resolve().parents[1]))


class HTTPJourney(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='casa-http-')
        self.db = str(Path(self.temp.name) / 'orders.db')
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0)); self.port = sock.getsockname()[1]
        self.url = f'http://127.0.0.1:{self.port}'
        self.client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
        self.start()

    def start(self):
        self.process = subprocess.Popen([sys.executable, str(ROOT/'app.py'), '--port', str(self.port), '--db', self.db],
            env={**os.environ, 'CASA_PIZZA_RESTAURANT_PASSWORD':'test-local-only'}, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(150):
            try:
                if self.request('/api/catalog')[0] == 200: return
            except OSError: pass
            if self.process.poll() is not None: self.fail('Le serveur est sorti avant le démarrage')
            time.sleep(.04)
        self.fail('Le serveur ne démarre pas')

    def stop(self):
        if self.process.poll() is None:
            self.process.terminate(); self.process.wait(timeout=10)

    def tearDown(self):
        self.stop(); self.temp.cleanup()

    def request(self, path, data=None):
        request = urllib.request.Request(self.url+path, data=json.dumps(data).encode() if data is not None else None,
            headers={'Content-Type':'application/json'} if data is not None else {})
        try: response=self.client.open(request,timeout=3)
        except urllib.error.HTTPError as e: response=e
        with response:
            content=response.read()
            return response.code, json.loads(content) if 'application/json' in response.headers.get('Content-Type','') else content

    def order(self, items=None):
        return {'customer_name':'Camille Atelier','address':'12 rue de la Démo, Ville Atelier','phone':'0000000000',
            'items':items or [{'pizza_id':'margherita','quantity':1},{'pizza_id':'reine','quantity':1}]}

    def create(self):
        code,data=self.request('/api/orders',self.order());self.assertEqual(code,201);return data

    def login(self):
        self.assertEqual(self.request('/api/restaurant/login',{'password':'test-local-only'})[0],200)

    def test_catalog_and_static_files(self):
        code,pizzas=self.request('/api/catalog');self.assertEqual(code,200);self.assertEqual(len(pizzas),6)
        self.assertEqual(sum(x['vegetarian'] for x in pizzas),3)
        self.assertEqual(self.request('/')[0],200)
        self.assertEqual(self.request('/static/style.css')[0],200)
        self.assertEqual(self.request('/static/../app.py')[0],404)

    def test_minimum_and_invalid_items(self):
        for items in ([{'pizza_id':'margherita','quantity':1}], [{'pizza_id':'inconnu','quantity':1}],
                      [{'pizza_id':'reine','quantity':0}], [{'pizza_id':'reine','quantity':True}]):
            self.assertEqual(self.request('/api/orders',self.order(items))[0],400)
        body=self.order();body['address']='';self.assertEqual(self.request('/api/orders',body)[0],400)

    def test_server_owns_prices(self):
        body=self.order();body['total_cents']=1;body['items'][0]['price_cents']=1
        code,data=self.request('/api/orders',body)
        self.assertEqual(code,201);self.assertEqual(data['subtotal_cents'],2200);self.assertEqual(data['total_cents'],2490)

    def test_tracking_is_private(self):
        order=self.create();path='/api/orders/'+order['number']
        self.assertEqual(self.request(path)[0],404);self.assertEqual(self.request(path+'?token=incorrect')[0],404)
        code,data=self.request(path+'?token='+order['tracking_token']);self.assertEqual(code,200);self.assertEqual(data['status'],'recue')
        self.assertNotIn('tracking_hash',data)

    def test_restaurant_authentication(self):
        order=self.create()
        self.assertEqual(self.request('/api/restaurant/orders')[0],401)
        self.assertEqual(self.request('/api/restaurant/login',{'password':'incorrect'})[0],401)
        self.assertEqual(self.request('/api/restaurant/orders/'+order['number']+'/status',{'status':'preparation'})[0],401)
        self.login();code,orders=self.request('/api/restaurant/orders');self.assertEqual(code,200);self.assertEqual(len(orders),1)

    def test_complete_status_chain(self):
        order=self.create();self.login();path='/api/restaurant/orders/'+order['number']+'/status'
        self.assertEqual(self.request(path,{'status':'livraison'})[0],409)
        for status in ['preparation','livraison','livree']:
            self.assertEqual(self.request(path,{'status':status})[0],200)
            self.assertEqual(self.request('/api/orders/'+order['number']+'?token='+order['tracking_token'])[1]['status'],status)
        self.assertEqual(self.request(path,{'status':'preparation'})[0],409)

    def test_real_process_restart_preserves_order(self):
        order=self.create();self.login();self.stop();self.start()
        code,data=self.request('/api/orders/'+order['number']+'?token='+order['tracking_token'])
        self.assertEqual(code,200);self.assertEqual(data['total_cents'],2490)
        self.assertEqual(self.request('/api/restaurant/orders')[0],401)
        self.login();self.assertEqual(len(self.request('/api/restaurant/orders')[1]),1)


if __name__=='__main__':
    unittest.main(verbosity=2)
