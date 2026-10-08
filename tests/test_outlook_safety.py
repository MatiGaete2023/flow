from __future__ import annotations

import sys
import types

import pytest

from nurus.adapters import outlook as outlook_adapter
from nurus.domain.models import Product, ProductKind, ProductStatus, Template
from nurus.services.rendering import prepare


def _product_without_recipient() -> Product:
    template = Template(
        "mail", "Correo", ProductKind.EMAIL,
        "Ingreso {TRIBUNAL}", "Hola {PROGRAMA}",
        ("TRIBUNAL", "PROGRAMA"), "published"
    )
    return prepare(
        Product(
            ProductKind.EMAIL,
            template,
            {"TRIBUNAL": "Laja", "PROGRAMA": "PRM"},
        )
    )


def test_save_draft_requires_explicit_confirmation():
    product = _product_without_recipient()
    with pytest.raises(ValueError, match="Confirma explícitamente"):
        outlook_adapter.save_draft(product)


def test_attachment_copy_keeps_exact_reviewed_bytes_and_original_name(tmp_path):
    from hashlib import sha256
    source = tmp_path / "nomina.xlsx"
    source.write_bytes(b"reviewed bytes")
    with outlook_adapter._reviewed_attachments([(str(source), sha256(source.read_bytes()).hexdigest())]) as copies:
        source.write_bytes(b"changed after copying")
        assert copies[0].read_bytes() == b"reviewed bytes"
        assert copies[0].name == source.name
        assert copies[0] != source
    assert not copies[0].exists()
    assert source.read_bytes() == b"changed after copying"


def test_changed_attachment_is_rejected(tmp_path):
    source = tmp_path / "nomina.xlsx"
    source.write_bytes(b"changed")
    with pytest.raises(ValueError, match="adjunto cambió"):
        with outlook_adapter._reviewed_attachments([(str(source), "wrong hash")]):
            pytest.fail("No debe entregar bytes no aprobados a Outlook")


@pytest.mark.parametrize("missing_id", [None, "entry", "store"])
def test_create_draft_allows_empty_recipient_and_never_sends(monkeypatch, missing_id):
    product = _product_without_recipient()
    assert product.status is ProductStatus.READY

    class FakeMail:
        def __init__(self):
            self.To = None
            self.CC = None
            self.Subject = None
            self.Body = None
            self.EntryID = "ENTRY-1"
            self.saved = False
            self.send_called = False

        def Save(self):
            self.saved = True

        def Send(self):
            self.send_called = True
            raise AssertionError("NuRus no puede llamar Send")

    mail = FakeMail()

    class FakeItems:
        def Add(self, message_class):
            assert message_class == "IPM.Note"
            return mail

    class FakeStore:
        StoreID = "STORE-1"

    class FakeFolder:
        Items = FakeItems()
        Store = FakeStore()
        FolderPath = r"\\Cuenta\Borradores"
        Folders = ()

    folder = FakeFolder()

    class FakeSession:
        Accounts = ()

        def GetDefaultFolder(self, folder_id):
            assert folder_id == 16
            return folder

    class FakeOutlook:
        Session = FakeSession()

    client_module = types.ModuleType("win32com.client")
    client_module.Dispatch = lambda name: FakeOutlook()
    win32com_module = types.ModuleType("win32com")
    win32com_module.client = client_module
    pythoncom_module = types.ModuleType("pythoncom")
    pythoncom_module.CoInitialize = lambda: None
    pythoncom_module.CoUninitialize = lambda: None
    monkeypatch.setitem(sys.modules, "win32com", win32com_module)
    monkeypatch.setitem(sys.modules, "win32com.client", client_module)
    monkeypatch.setitem(sys.modules, "pythoncom", pythoncom_module)
    monkeypatch.setattr(outlook_adapter.platform, "system", lambda: "Windows")

    # Defensa al guardar aunque alguien haya vaciado CC tras preparar el texto.
    product.cc = ""
    if missing_id:
        if missing_id == "entry":
            mail.EntryID = ""
        else:
            folder.Store.StoreID = ""
        with pytest.raises(outlook_adapter.DraftSaveUncertain):
            outlook_adapter.save_draft(product, confirmed=True)
        assert mail.saved and not mail.send_called
        return
    receipt = outlook_adapter.save_draft(product, confirmed=True)

    assert receipt.entry_id == "ENTRY-1"
    assert receipt.store_id == "STORE-1"
    assert mail.saved is True
    assert mail.send_called is False
    assert mail.To == ""
    assert mail.CC == "ucc_concepcion@pjud.cl"


def test_account_selection_is_exact_and_sets_send_using_account_without_sending(monkeypatch):
    product = _product_without_recipient()

    class FakeMail:
        EntryID = "ENTRY-2"
        SendUsingAccount = None

        def Save(self):
            self.saved = True

    mail = FakeMail()

    class FakeItems:
        def Add(self, _message_class):
            return mail

    class FakeFolder:
        Items = FakeItems()
        Store = types.SimpleNamespace(StoreID="STORE-2")
        FolderPath = r"\\Cuenta\Borradores"
        Folders = ()

    folder = FakeFolder()

    class FakeDeliveryStore:
        def GetDefaultFolder(self, folder_id):
            assert folder_id == 16
            return folder

    account = types.SimpleNamespace(
        SmtpAddress="cuenta@example.test",
        DisplayName="Cuenta institucional",
        DeliveryStore=FakeDeliveryStore(),
    )
    session = types.SimpleNamespace(Accounts=[account])
    outlook = types.SimpleNamespace(Session=session)

    client_module = types.ModuleType("win32com.client")
    client_module.Dispatch = lambda _name: outlook
    win32com_module = types.ModuleType("win32com")
    win32com_module.client = client_module
    pythoncom_module = types.ModuleType("pythoncom")
    pythoncom_module.CoInitialize = lambda: None
    pythoncom_module.CoUninitialize = lambda: None
    monkeypatch.setitem(sys.modules, "win32com", win32com_module)
    monkeypatch.setitem(sys.modules, "win32com.client", client_module)
    monkeypatch.setitem(sys.modules, "pythoncom", pythoncom_module)
    monkeypatch.setattr(outlook_adapter.platform, "system", lambda: "Windows")

    outlook_adapter.save_draft(
        product,
        confirmed=True,
        account_key="cuenta@example.test",
    )
    assert mail.SendUsingAccount is account


@pytest.mark.parametrize("cc,expected", [
    ("", "ucc_concepcion@pjud.cl"),
    ("otro@example.test", "otro@example.test; ucc_concepcion@pjud.cl"),
    ("UCC_CONCEPCION@PJUD.CL; ucc_concepcion@pjud.cl", "UCC_CONCEPCION@PJUD.CL"),
    ("uno@example.test, dos@example.test", "uno@example.test; dos@example.test; ucc_concepcion@pjud.cl"),
])
def test_mandatory_copy_preserves_other_recipients(cc, expected):
    from nurus.services.policy import with_mandatory_cc
    assert with_mandatory_cc(cc) == expected
    assert with_mandatory_cc(expected) == expected


def test_preview_includes_mandatory_copy():
    product = _product_without_recipient()
    assert product.recipient == ""
    assert product.cc == "ucc_concepcion@pjud.cl"


def test_resolution_cannot_create_mail():
    product = _product_without_recipient()
    product.kind = ProductKind.RESOLUTION
    with pytest.raises(ValueError, match="Solo un producto"):
        outlook_adapter.save_draft(product, confirmed=True)
