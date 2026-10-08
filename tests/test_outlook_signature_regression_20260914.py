import sys
import types

from nurus.adapters import outlook as outlook_adapter
from nurus.domain.models import Product, ProductKind, ProductStatus, Template


def test_preserved_signature_closes_inspector_without_sending(monkeypatch):
    template=Template('mail','Correo',ProductKind.EMAIL,'','','')
    product=Product(ProductKind.EMAIL,template,{},recipient='',cc='',status=ProductStatus.READY,
                    rendered_subject='Prueba',rendered_body='Cuerpo revisado')

    class FakeAttachments:
        def Add(self,_path):
            raise AssertionError('La prueba no requiere adjuntos')

    class FakeMail:
        def __init__(self):
            self.To='';self.CC='';self.Subject='';self.HTMLBody='<html><body><p>Firma institucional</p></body></html>'
            self.EntryID='ENTRY-SIGN';self.Attachments=FakeAttachments();self.displayed=False;self.saved=False;self.closed=[];self.sent=False
        def Display(self,modal):
            assert modal is False;self.displayed=True
        def Save(self):self.saved=True
        def Close(self,mode):self.closed.append(mode)
        def Send(self):self.sent=True;raise AssertionError('No puede enviar')

    mail=FakeMail()
    folder=types.SimpleNamespace(
        Items=types.SimpleNamespace(Add=lambda _kind:mail),
        Store=types.SimpleNamespace(StoreID='STORE-SIGN'),
        FolderPath=r'\\Cuenta\Borradores',Folders=())
    session=types.SimpleNamespace(Accounts=(),GetDefaultFolder=lambda folder_id:folder)
    outlook=types.SimpleNamespace(Session=session)

    client_module=types.ModuleType('win32com.client');client_module.Dispatch=lambda _name:outlook
    win32com_module=types.ModuleType('win32com');win32com_module.client=client_module
    pythoncom_module=types.ModuleType('pythoncom');pythoncom_module.CoInitialize=lambda:None;pythoncom_module.CoUninitialize=lambda:None
    monkeypatch.setitem(sys.modules,'win32com',win32com_module)
    monkeypatch.setitem(sys.modules,'win32com.client',client_module)
    monkeypatch.setitem(sys.modules,'pythoncom',pythoncom_module)
    monkeypatch.setattr(outlook_adapter.platform,'system',lambda:'Windows')

    receipt=outlook_adapter.save_draft(product,confirmed=True,preserve_signature=True)

    assert receipt.entry_id=='ENTRY-SIGN'
    assert mail.displayed and mail.saved and mail.closed==[0]
    assert not mail.sent
    assert 'Cuerpo revisado' in mail.HTMLBody
    assert 'Firma institucional' in mail.HTMLBody
