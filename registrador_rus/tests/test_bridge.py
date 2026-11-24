from dataclasses import replace
from hashlib import sha256
import http.client
import json
import unittest

from registrador_rus.app.bridge import LocalBridge, BridgeError, Mailbox
from registrador_rus.app.contracts import RegistroFlowV1, ContractError
from test_contracts import sample

ORIGIN = 'chrome-extension://' + 'a'*32


class BridgeTests(unittest.TestCase):
    def setUp(self):
        self.bridge=LocalBridge(ORIGIN)
        self.box=self.bridge.mailbox
        self.op=RegistroFlowV1.from_mapping(sample())

    def tearDown(self):
        self.bridge.close()

    def post(self, path, data, **overrides):
        headers={'Host':self.bridge.host,'Origin':ORIGIN,'Authorization':'Bearer '+self.bridge.token,'Content-Type':'application/json'}
        headers.update(overrides)
        c=http.client.HTTPConnection(self.bridge.host,timeout=3)
        try:
            c.request('POST',path,body=data if isinstance(data,str) else json.dumps(data),headers=headers)
            r=c.getresponse(); body=r.read()
            return r.status,json.loads(body) if body else None
        finally:
            c.close()

    def poll(self,**headers):
        return self.post('/v1/poll',dict(schema='BridgePollV1',session_id=self.box.session_id),**headers)

    def test_mailbox_and_approval(self):
        with self.assertRaises(BridgeError): self.box.submit('write_one',frozen=self.op,current=self.op)
        with self.assertRaises(ContractError): self.box.submit('prepare',frozen=self.op,current=replace(self.op,destination='AL_TRIBUNAL'))
        key=self.box.submit('prepare',frozen=self.op,current=self.op)
        with self.assertRaises(BridgeError): self.box.submit('get_capabilities')
        status,body=self.poll();self.assertEqual(status,200)
        self.assertEqual(body['command']['command_id'],key);self.assertTrue(body['command']['simulation_only'])
        self.assertEqual(body['command']['approved_operation']['approved_text'],self.op.approved_text)
        self.assertEqual(self.poll(),(200,{'command':None}))
        ack=dict(schema='BridgeAckV1',session_id=self.box.session_id,command_id=key,status='OK')
        self.assertEqual(self.post('/v1/ack',ack),(200,{'accepted':True}))
        self.assertEqual(self.post('/v1/ack',ack)[0],400)
        with self.assertRaises(BridgeError): self.box.submit('get_capabilities')
        self.assertEqual(self.box.take_result()['status'],'OK')
        self.assertFalse(self.bridge.capabilities()['write_one'])

    def test_origin_host_token_session(self):
        for headers in ({'Origin':'https://example.invalid'},{'Host':'localhost'},{'Authorization':'Bearer incorrecto'}):
            self.assertEqual(self.poll(**headers)[0],403)
        self.assertEqual(self.post('/v1/poll',dict(schema='BridgePollV1',session_id='otra'))[0],400)
        self.assertEqual(self.post('/v1/poll',dict(schema='BridgePollV1',session_id=self.box.session_id,write_one=True))[0],400)

    def test_command_size_and_preflight(self):
        text='x'*70000
        long_op=replace(self.op,approved_text=text,approved_text_sha256=sha256(text.encode()).hexdigest())
        with self.assertRaisesRegex(BridgeError,'COMANDO_EXCEDE_TRANSPORTE'):
            self.box.submit('prepare',frozen=long_op,current=long_op)
        c=http.client.HTTPConnection(self.bridge.host,timeout=3)
        try:
            c.request('OPTIONS','/v1/poll',headers={'Host':self.bridge.host,'Origin':ORIGIN,
                'Access-Control-Request-Method':'POST','Access-Control-Request-Headers':'authorization, content-type'})
            r=c.getresponse();self.assertEqual(r.status,204);self.assertEqual(r.getheader('Access-Control-Allow-Origin'),ORIGIN);r.read()
        finally:
            c.close()

    def test_closed_protocol(self):
        self.assertEqual(self.post('/v1/write_one',{})[0],404)
        self.assertEqual(self.post('/v1/poll','{"schema":"BridgePollV1","schema":"BridgePollV1","session_id":"x"}')[0],400)
        self.assertEqual(self.post('/v1/poll','x'*5000)[0],400)
        key=self.box.submit('inspect_current',frozen=self.op,current=self.op)
        status,body=self.poll();self.assertEqual(status,200)
        self.assertNotIn('approved_operation',body['command'])
        self.assertEqual(self.post('/v1/ack',dict(schema='BridgeAckV1',session_id=self.box.session_id,command_id='otro',status='OK'))[0],400)
        self.assertEqual(self.post('/v1/ack',dict(schema='BridgeAckV1',session_id=self.box.session_id,command_id=key,status='BLOCKED'))[0],200)
