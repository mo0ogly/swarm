"""A raw HEAD witness must reject bytes hidden by HTTPResponse.read()."""
import socket,threading,unittest
from rust_native_http_cli import assert_head_wire

class HeadWireTests(unittest.TestCase):
    """Owned one-request TCP fixtures, no provider, credential or global watchdog."""
    def witness(self,body,connection='close',mime='application/json'):
        """Serve exact positive/framing-counterexample bytes, joining owned thread."""
        listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen(1);port=listener.getsockname()[1];errors=[]
        def serve():
            try:
                with listener.accept()[0] as stream:
                    request=b''
                    while b'\r\n\r\n' not in request:request+=stream.recv(65536)
                    stream.sendall(('HTTP/1.1 200 OK\r\nContent-Type: '+mime+'\r\nConnection: '+connection+'\r\n\r\n').encode('ascii')+body)
            except BaseException as e:errors.append(e)
        thread=threading.Thread(target=serve);thread.start()
        try:return assert_head_wire('127.0.0.1',port,'/owned','fixture-only')
        finally:
            thread.join();listener.close()
            if errors:raise BaseExceptionGroup('Owned HEAD server failed',errors)
    def test_valid_empty_wire_body(self):
        self.assertEqual(self.witness(b'')['body_bytes'],0)
    def test_nonempty_wire_body_is_rejected(self):
        # Valid witness precedes the injected common-client-blind counterexample.
        self.assertEqual(self.witness(b'')['status'],200)
        with self.assertRaisesRegex(AssertionError,'unexpectedly published body'):self.witness(b'[]')
    def test_explicit_keepalive_refused(self):
        self.assertEqual(self.witness(b'')['status'],200)
        with self.assertRaisesRegex(AssertionError,'ignored Connection: close'):self.witness(b'',connection='keep-alive')
    def test_media_type_exact_and_case_insensitive(self):
        self.assertEqual(self.witness(b'',mime='APPLICATION/JSON; charset=UTF-8')['status'],200)
        with self.assertRaisesRegex(AssertionError,'MIME mismatch'):self.witness(b'',mime='application/jsonx')
    def test_host_header_injection_refused_before_socket(self):
        with self.assertRaisesRegex(ValueError,'header-injection'):assert_head_wire('127.0.0.1\r\nX: 1',0,'/owned','fixture-only')
if __name__=='__main__':unittest.main(verbosity=2)
