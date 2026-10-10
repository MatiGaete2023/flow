"""Barra «Siguiente paso»: solo lee el estado del trabajo y de los productos; no crea datos ni reglas."""
from pathlib import Path

from . import ui
from .product_checks import check_products, current_drafts
from .ux_support import incident_ids

STEPS = (
    ('revisar', 'Revisar', 'Trabajo'),
    ('exportar', 'Exportar copia', 'Trabajo'),
    ('correos', 'Correos', 'Correos'),
    ('resoluciones', 'Resoluciones', 'Resoluciones'),
    ('resultados', 'Resultados', 'Resultados'),
)
INTERVAL_MS = 1500


def counts(work, drafts, projects):
    """Los mismos números que muestra cada página."""
    checks = check_products(work, drafts, projects) if work else []
    output = getattr(work, 'output', '') if work else ''
    return {
        'revisar': len(incident_ids(work)) if work else 0,
        'exportar': bool(output) and Path(output).is_file(),
        'correos': sum(c.kind == 'Correo' for c in checks),
        'resoluciones': sum(c.kind == 'Word' for c in checks),
        'resultados': sum(bool(c.issues) for c in checks),
    }


def labels(values):
    out = []
    for number, (key, name, _) in enumerate(STEPS, 1):
        if key == 'exportar':
            out.append(f'{number} {name}' + (' ✓' if values.get(key) else ''))
        else:
            out.append(f'{number} {name} ({values.get(key, 0)})')
    return out


def text(values):
    return ' → '.join(labels(values))


def _signature(app):
    work = getattr(app, 'work', None)
    drafts = current_drafts(app) if hasattr(app, 'drafts') else []
    projects = list(getattr(app, 'projects', []))
    if not work:
        return None
    output = getattr(work, 'output', '')
    return (id(work), len(work.rows), sum(bool(r.warnings) for r in work.rows), output,
            Path(output).is_file() if output else False, len(getattr(work, 'receipts', {}) or {}),
            tuple((d.product_id, d.subject, bool(d.to), bool(d.body.strip()), len(d.attachments), d.roster_reviewed)
                  for d in drafts),
            tuple((p.key, p.review_required) for p in projects))


class NextStepBar(ui.Frame):
    """Cada paso abre su página. Un único temporizador recalcula solo si cambió el estado."""

    def __init__(self, parent, app):
        super().__init__(parent)
        self.app = app
        self.values = {}
        self._signature = object()
        self._after = None
        ui.Label(self, text='Siguiente paso', text_color=ui.MUTED).pack(side='left', padx=(10, 6))
        self.buttons = {}
        for index, (key, name, page) in enumerate(STEPS):
            if index:
                ui.Label(self, text='→', text_color=ui.MUTED).pack(side='left', padx=2)
            button = ui.Button(self, text=name, width=60, height=26, fg_color='transparent', border_width=1,
                               command=lambda page=page: self.open(page))
            button.pack(side='left', padx=2, pady=3)
            self.buttons[key] = button
        self.refresh(force=True)
        self._schedule()

    def open(self, page):
        self.app.tabs.select(self.app.pages[page])

    def refresh(self, force=False):
        signature = _signature(self.app)
        if not force and signature == self._signature:
            return False
        self._signature = signature
        app = self.app
        work = getattr(app, 'work', None)
        self.values = counts(work, current_drafts(app) if work else [], list(getattr(app, 'projects', [])))
        for (key, _, _), label in zip(STEPS, labels(self.values), strict=True):
            self.buttons[key].configure(text=label)
        return True

    def _tick(self):
        self._after = None
        try:
            if not (getattr(self.app, 'busy', False) or getattr(self.app, 'rendering', False)):
                self.refresh()
        finally:
            self._schedule()

    def _schedule(self):
        if self._after is None and self.winfo_exists():
            self._after = self.after(INTERVAL_MS, self._tick)

    def destroy(self):
        if self._after is not None:
            try:
                self.after_cancel(self._after)
            except Exception:
                pass
            self._after = None
        super().destroy()
