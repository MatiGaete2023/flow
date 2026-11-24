"""Prueba DPAPI nativa en Windows; en otras plataformas exige bloqueo."""
from dataclasses import replace
from pathlib import Path
from tempfile import TemporaryDirectory
import os
import subprocess
import sys
from unittest import TestCase, skipUnless
from unittest.mock import patch

from registrador_rus.app.contracts import ContractError
from registrador_rus.app.demo_transfer import make_demo_batch
from registrador_rus.app.protected_demo_transfer import (
    protect_demo, unprotect_demo, export_protected_demo, import_protected_demo,
    DESCRIPTION, ENTROPY, UI_FORBIDDEN, MAX_CIPHERTEXT,
)


class ProtectedDemoTests(TestCase):
    def test_invalid_envelope_and_platform_gate(self):
        for bad in (b'',b'x'*(MAX_CIPHERTEXT+1),'texto'):
            with self.assertRaises(ContractError):unprotect_demo(bad)
        with patch('registrador_rus.app.protected_demo_transfer.os.name','posix'):
            with self.assertRaisesRegex(ContractError,'DPAPI_REQUIERE_WINDOWS'):protect_demo(1)

    @skipUnless(os.name=='nt','DPAPI nativa solo Windows')
    def test_native_round_trip_400_across_processes(self):
        with TemporaryDirectory() as tmp:
            target=Path(tmp)/'lote.flow-demo.dpapi'
            result=subprocess.run([sys.executable,'-m','registrador_rus.app.protected_demo_transfer',str(target),'--count','400'],
                                  capture_output=True,timeout=30)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(result.stdout,b'')
            ciphertext=target.read_bytes()
            self.assertNotIn(b'Constancia ficticia',ciphertext)
            self.assertNotIn(b'RegistroFlowV1',ciphertext)
            self.assertEqual(import_protected_demo(target),make_demo_batch(400))
            self.assertNotEqual(ciphertext,protect_demo(400))
            with self.assertRaises(FileExistsError):export_protected_demo(target,8)
            self.assertEqual(target.read_bytes(),ciphertext)
            self.assertEqual(list(Path(tmp).iterdir()),[target])
            for tampered in (ciphertext[:len(ciphertext)//2],ciphertext[:-1]+bytes([ciphertext[-1]^1]),make_demo_batch(1).encode()):
                with self.assertRaises(ContractError):unprotect_demo(tampered)

    @skipUnless(os.name=='nt','DPAPI nativa solo Windows')
    def test_native_rejects_valid_encrypted_non_demo(self):
        import win32crypt
        batch=make_demo_batch(2)
        other=replace(batch,revision=1)
        encrypted=win32crypt.CryptProtectData(other.encode(),DESCRIPTION,ENTROPY,None,None,UI_FORBIDDEN)
        with self.assertRaisesRegex(ContractError,'SOLO_LOTE_FICTICIO_GENERADO'):unprotect_demo(encrypted)
        wrong_purpose=win32crypt.CryptProtectData(batch.encode(),'Otro propósito',ENTROPY,None,None,UI_FORBIDDEN)
        with self.assertRaisesRegex(ContractError,'SOBRE_DPAPI_INVALIDO'):unprotect_demo(wrong_purpose)

    @skipUnless(os.name=='nt','DPAPI nativa solo Windows')
    def test_consumer_startup_and_rejection_before_tk(self):
        from registrador_rus.app.main import main
        with TemporaryDirectory() as tmp:
            target=Path(tmp)/'lote.flow-demo.dpapi'
            export_protected_demo(target,8)
            with patch.object(sys,'argv',['registrador','--protected-demo-batch',str(target)]), \
                 patch('registrador_rus.app.main.tk.Tk') as root,patch('registrador_rus.app.main.DemoApp') as app:
                main()
                app.assert_called_once_with(root.return_value,demo_batch=make_demo_batch(8))
            target.write_bytes(b'not encrypted')
            with patch.object(sys,'argv',['registrador','--protected-demo-batch',str(target)]), \
                 patch('registrador_rus.app.main.tk.Tk') as root,self.assertRaises(SystemExit) as stopped:
                main()
            self.assertEqual(stopped.exception.code,2)
            root.assert_not_called()
