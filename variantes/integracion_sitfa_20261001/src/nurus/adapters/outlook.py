from __future__ import annotations

import platform
from dataclasses import dataclass
from contextlib import contextmanager
from hashlib import sha256
from pathlib import Path
import tempfile

from nurus.domain.models import Product, ProductStatus, ProductKind
from nurus.services.policy import with_mandatory_cc


class OutlookUnavailable(RuntimeError):
    pass


class DraftSaveUncertain(RuntimeError):
    """El guardado pudo producirse pero no se obtuvo confirmación suficiente."""


@contextmanager
def _reviewed_attachments(attachments):
    """Entrega a Outlook exactamente los bytes cuyo hash aprobó la persona."""
    with tempfile.TemporaryDirectory(prefix="nurus-adjuntos-") as directory:
        copies = []
        for index, (path, digest) in enumerate(attachments):
            source = Path(path)
            target = Path(directory) / str(index) / source.name
            target.parent.mkdir()
            calculated = sha256()
            with source.open("rb") as input_file, target.open("xb") as output:
                for block in iter(lambda: input_file.read(1024 * 1024), b""):
                    calculated.update(block)
                    output.write(block)
            if calculated.hexdigest() != digest:
                raise ValueError("El adjunto cambió después de la revisión. Prepara una nueva versión.")
            copies.append(target)
        yield copies


@dataclass(frozen=True)
class DraftReceipt:
    entry_id: str
    store_id: str
    account: str
    folder: str


def _account_key(account) -> str:
    for attribute in ("SmtpAddress", "DisplayName"):
        try:
            value = str(getattr(account, attribute, "") or "").strip()
        except Exception:
            value = ""
        if value:
            return value
    return ""


def _find_account(session, requested: str):
    key = requested.strip().casefold()
    matches = []
    for account in session.Accounts:
        values = []
        for attribute in ("SmtpAddress", "DisplayName"):
            try:
                values.append(str(getattr(account, attribute, "") or "").strip())
            except Exception:
                pass
        if any(value.casefold() == key for value in values if value):
            matches.append(account)
    if len(matches) != 1:
        raise OutlookUnavailable(
            "No existe una cuenta Outlook única que coincida exactamente con la cuenta seleccionada."
        )
    return matches[0]


def _child_folder(folder, name: str):
    wanted = name.strip().casefold()
    matches = []
    for child in folder.Folders:
        try:
            child_name = str(child.Name or "").strip()
        except Exception:
            continue
        if child_name.casefold() == wanted:
            matches.append(child)
    if len(matches) != 1:
        raise OutlookUnavailable(f"No existe una carpeta Outlook única llamada {name!r}.")
    return matches[0]


def save_draft(
    product: Product,
    *,
    confirmed: bool = False,
    account_key: str | None = None,
    folder_path: tuple[str, ...] = (),
    preserve_signature: bool = False,
) -> DraftReceipt:
    """Guarda un borrador Outlook y devuelve su identidad.

    INVARIANTE: esta función no envía correos. `account_key` selecciona una
    cuenta por dirección SMTP o nombre visible exactos. `folder_path`, si se
    entrega, se resuelve debajo de la carpeta Borradores de la cuenta elegida.
    """
    if not confirmed:
        raise ValueError("Confirma explícitamente la creación del borrador Outlook.")
    if product.kind is not ProductKind.EMAIL:
        raise ValueError("Solo un producto de correo puede crear un borrador Outlook.")
    if product.required_attachment and not product.attachments:
        raise ValueError("El tipo de correo requiere un adjunto.")
    if product.status not in {ProductStatus.READY, ProductStatus.APPROVED}:
        raise ValueError("Solo se pueden crear borradores de productos completos y revisados.")
    if platform.system() != "Windows":
        raise OutlookUnavailable("Outlook Object Model solo se prueba en Windows con Outlook clásico.")

    try:
        import pythoncom  # type: ignore[import-not-found]
        import win32com.client  # type: ignore[import-not-found]
    except ImportError as exc:
        raise OutlookUnavailable("Instale pywin32 en el equipo Windows autorizado.") from exc

    save_attempted = False
    pythoncom.CoInitialize()
    try:
        outlook = win32com.client.Dispatch("Outlook.Application")
        session = outlook.Session
        account = _find_account(session, account_key) if account_key else None
        # olFolderDrafts = 16. Usamos el store de la cuenta seleccionada cuando existe.
        if account is not None:
            delivery_store = getattr(account, "DeliveryStore", None)
            if delivery_store is None:
                raise OutlookUnavailable("La cuenta seleccionada no expone un almacén de entrega.")
            folder = delivery_store.GetDefaultFolder(16)
        else:
            folder = session.GetDefaultFolder(16)

        for part in folder_path:
            if part.strip():
                folder = _child_folder(folder, part)

        mail = folder.Items.Add("IPM.Note")
        if account is not None:
            mail.SendUsingAccount = account
        mail.To = product.recipient
        mail.CC = with_mandatory_cc(product.cc)
        mail.Subject = product.rendered_subject
        if preserve_signature:
            from html import escape
            mail.Display(False)
            signature = str(mail.HTMLBody or '')
            addition = '<div>' + escape(product.rendered_body).replace('\n', '<br>') + '</div><br>'
            import re
            match = re.search(r'<body\b[^>]*>', signature, flags=re.IGNORECASE)
            mail.HTMLBody = (signature[:match.end()] + addition + signature[match.end():]) if match else addition + signature
        else:
            mail.Body = product.rendered_body
        with _reviewed_attachments(product.attachments) as attachments:
            for path in attachments:
                mail.Attachments.Add(str(path))

            # INVARIANTE D01: el único efecto externo permitido es guardar un borrador.
            save_attempted = True
            mail.Save()
        try:
            entry_id = str(getattr(mail, "EntryID", "") or "")
            store = getattr(folder, "Store", None)
            store_id = str(getattr(store, "StoreID", "") or "") if store is not None else ""
            folder_name = str(getattr(folder, "FolderPath", "") or getattr(folder, "Name", "") or "")
            if not entry_id.strip() or not store_id.strip():
                raise DraftSaveUncertain("Outlook no devolvió EntryID y StoreID completos; concilia antes de reintentar.")
        except Exception as exc:
            raise DraftSaveUncertain(
                "El borrador fue guardado, pero no se pudo confirmar completamente su identidad."
            ) from exc
        # Display() es necesario para que Outlook inserte su firma, pero no debe
        # dejar un Inspector abierto por cada elemento de un lote. El borrador ya
        # fue guardado y validado; un fallo al cerrar la ventana no invalida ese guardado.
        if preserve_signature:
            try:
                mail.Close(0)  # olSave
            except Exception:
                pass
        return DraftReceipt(
            entry_id=entry_id,
            store_id=store_id,
            account=_account_key(account) if account is not None else "predeterminada",
            folder=folder_name,
        )
    except DraftSaveUncertain:
        raise
    except Exception as exc:
        if save_attempted:
            raise DraftSaveUncertain(
                "Outlook pudo haber guardado el borrador; no reintentes automáticamente hasta conciliar."
            ) from exc
        if isinstance(exc, (OutlookUnavailable, ValueError)):
            raise
        raise OutlookUnavailable(f"No se pudo preparar el borrador Outlook: {exc}") from exc
    finally:
        pythoncom.CoUninitialize()
