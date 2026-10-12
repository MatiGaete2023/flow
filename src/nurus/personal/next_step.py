"""Barra «Siguiente paso»: solo lee el estado del trabajo y de los productos; no crea datos ni reglas."""

from pathlib import Path
import json

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


def counts(work, drafts, projects, checks=None):
    """Los mismos números que muestra cada página."""
    checks = (check_products(work, drafts, projects) if work else []) if checks is None else checks
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


def _file_state(path):
    """Detecta desapariciones y cambios de archivos sin leer su contenido en cada pulso."""
    if not path:
        return None
    try:
        info = Path(path).stat()
    except FileNotFoundError:
        return (str(path), None)
    return (str(path), info.st_mtime_ns, info.st_size)


def _signature(app):
    work = getattr(app, 'work', None)
    drafts = current_drafts(app) if hasattr(app, 'drafts') else []
    projects = list(getattr(app, 'projects', []))
    if not work:
        return None
    output = getattr(work, 'output', '')
    return (
        id(work),
        len(work.rows),
        sum(bool(r.warnings) for r in work.rows),
        output,
        Path(output).is_file() if output else False,
        json.dumps(getattr(work, 'receipts', {}) or {}, sort_keys=True, default=str),
        tuple(_file_state(r.get('path')) for r in (getattr(work, 'receipts', {}) or {}).values()),
        tuple(
            (
                d.product_id,
                d.subject,
                d.to,
                d.cc,
                d.body,
                d.required,
                tuple(_file_state(path) for path in d.attachments),
                d.roster_reviewed,
                tuple(_file_state(group.get('generated')) for group in d.roster),
            )
            for d in drafts
        ),
        tuple(
            (
                p.key,
                p.review_required,
                p.text,
                json.dumps(p.values, sort_keys=True, default=str),
                _file_state(p.template),
            )
            for p in projects
        ),
    )


class NextStepBar(ui.Frame):
    """Cada paso abre su página. Un único temporizador recalcula solo si cambió el estado."""

    def __init__(self, parent, app):
        super().__init__(parent)
        self.app = app
        self.values = {}
        self._signature = object()
        self._after = None
        self.error = None
        flow = ui.Frame(self)
        flow.pack(fill='x')
        self.flow_labels = {}
        for key, name in (
            ('importar', '1 Importar'),
            ('revisar', '2 Revisar'),
            ('correos', '3 Correos'),
            ('resoluciones', '4 Resoluciones'),
        ):
            label = ui.Label(flow, text=name + '\nPendiente', width=90, font=('Segoe UI', 12))
            label.pack(side='left', padx=2)
            self.flow_labels[key] = label
        self.action_button = ui.Button(
            flow, text='Siguiente acción', width=165, role='secondary', command=self.next_action
        )
        self.action_button.pack(side='left', padx=6)
        ui.Button(flow, text='Detalle', width=70, role='secondary', command=self.toggle_details).pack(side='left')
        self.details = ui.Frame(self)
        ui.Label(self.details, text='Estado de productos', text_color=ui.MUTED).pack(side='left', padx=(6, 4))
        self.buttons = {}
        for index, (key, name, page) in enumerate(STEPS):
            if index:
                ui.Label(self.details, text='→', text_color=ui.MUTED).pack(side='left', padx=2)
            button = ui.Button(
                self.details,
                text=name,
                width=40,
                height=22,
                fg_color='transparent',
                border_width=1,
                command=lambda page=page: self.open(page),
            )
            button.pack(side='left', padx=1, pady=1)
            self.buttons[key] = button
        self.refresh(force=True)
        self._schedule()

    def open(self, page):
        self.app.tabs.select(self.app.pages[page])
        self.refresh(force=True)

    def toggle_details(self):
        if self.details.winfo_manager():
            self.details.pack_forget()
        else:
            self.details.pack(fill='x')

    def next_action(self):
        app = self.app
        if app.busy or getattr(app, 'rendering', False) or app.grab_current() is not None:
            app.status.set('Termina la operación o el diálogo antes de continuar.')
            return
        self.refresh(force=True)  # Una acción recién preparada no espera al siguiente pulso del temporizador.
        work = getattr(app, 'work', None)
        if not work:
            app._guard(app._import_capture)
        elif not self.values['exportar']:
            app._guard(app._export_current)
        elif app.tabs.select() == str(app.pages['Trabajo']):
            self.open('Correos')
            button = getattr(app, 'mail_action_buttons', {}).get('preparar_todos')
            if button:
                button._canvas.focus_set()
        elif app.tabs.select() == str(app.pages['Correos']):
            self.open('Resoluciones' if self.values['correos'] else 'Correos')
            if not self.values['correos']:
                app.mail_action_buttons['preparar_todos']._canvas.focus_set()
                app.status.set('Elige Preparar tipo o Preparar todos; revisa destinatarios antes de guardar.')
        elif app.tabs.select() == str(app.pages['Resoluciones']):
            self.open('Resultados' if self.values['resoluciones'] else 'Resoluciones')
            if not self.values['resoluciones']:
                app.prepare_word_button._canvas.focus_set()
                app.status.set('Elige los registros y pulsa Preparar / actualizar proyectos.')
        else:
            self.open('Trabajo')

    def refresh(self, force=False):
        signature = (_signature(self.app), self.app.tabs.select())
        if not force and signature == self._signature:
            return False
        app = self.app
        work = getattr(app, 'work', None)
        drafts = current_drafts(app) if work else []
        projects = list(getattr(app, 'projects', []))
        checks = check_products(work, drafts, projects) if work else []
        self.values = counts(work, drafts, projects, checks)
        for (key, _, _), label in zip(STEPS, labels(self.values), strict=True):
            self.buttons[key].configure(text=label)
        states = {'importar': 'Cargado' if work else 'Pendiente', 'revisar': 'Disponible' if work else 'Bloqueado'}
        if work and self.values['revisar']:
            states['revisar'] = 'Atención'
        for key, kind, completed in (
            ('correos', 'Correo', 'Guardado en Outlook'),
            ('resoluciones', 'Word', 'Generado'),
        ):
            products = [check for check in checks if check.kind == kind]
            states[key] = (
                'Bloqueado'
                if not self.values['exportar']
                else 'Disponible'
                if not products
                else 'Atención'
                if any(check.issues for check in products)
                else 'Completado'
                if all(check.state == completed for check in products)
                else 'Preparado'
            )
        for key, label in self.flow_labels.items():
            title = label.cget('text').split('\n')[0]
            badge = (
                'warning'
                if states[key] == 'Atención'
                else 'ok'
                if states[key] in ('Cargado', 'Completado')
                else 'pending'
            )
            label.configure(
                text=title + '\n' + states[key],
                fg_color=ui.COLORS[badge + '_background'],
                text_color=ui.COLORS[badge + '_text'],
                corner_radius=6,
            )
        active = app.tabs.select() if hasattr(app, 'tabs') else ''
        action = (
            'Importar paquete'
            if not work
            else 'Reintentar copia Excel'
            if not self.values['exportar']
            else 'Continuar a Correos'
            if active == str(app.pages['Trabajo'])
            else 'Continuar a Resoluciones'
            if active == str(app.pages['Correos']) and drafts
            else 'Revisar resultados'
            if active == str(app.pages['Resoluciones']) and projects
            else 'Revisar preparación'
            if active in (str(app.pages['Correos']), str(app.pages['Resoluciones']))
            else 'Revisar trabajo'
        )
        self.action_button.configure(
            text=action, state='disabled' if app.busy or getattr(app, 'rendering', False) else 'normal'
        )
        for attr, primary in (
            ('import_button', not work),
            ('work_primary_button', bool(work)),
            ('prepare_word_button', not projects),
            ('generate_word_button', bool(projects)),
        ):
            button = getattr(app, attr, None)
            if button:
                button.set_role('primary' if primary else 'secondary')
        self._signature = signature
        return True

    def _tick(self):
        self._after = None
        try:
            if not (getattr(self.app, 'busy', False) or getattr(self.app, 'rendering', False)):
                self.refresh()
            self.action_button.configure(
                state='disabled'
                if getattr(self.app, 'busy', False) or getattr(self.app, 'rendering', False)
                else 'normal'
            )
            self.error = None
        except Exception as exc:  # una ruta de red caída no debe repetir tracebacks cada 1,5 s
            if self.error is None:
                self.error = exc
                status = getattr(self.app, 'status', None)
                if status is not None:
                    status.set('No se pudieron actualizar los pasos: ' + str(exc)[:120])
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
