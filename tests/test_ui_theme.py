"""Contraste y compatibilidad de los temas compartidos en las dos interfaces."""

import os
from pathlib import Path
import subprocess
import sys

import pytest

from nurus.ui_theme import button_options, tokens
from test_integral_ui import run_gui


def luminance(color):
    values = [int(color[i : i + 2], 16) / 255 for i in (1, 3, 5)]
    linear = [v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in values]
    return sum(a * b for a, b in zip(linear, (0.2126, 0.7152, 0.0722)))


def ratio(first, second):
    low, high = sorted((luminance(first), luminance(second)))
    return (high + 0.05) / (low + 0.05)


@pytest.mark.parametrize('mode', ['Claro', 'Oscuro'])
def test_texto_y_estados_interactivos_tienen_contraste_aa(mode):
    palette = tokens(mode)
    pairs = [
        ('text', 'panel'),
        ('text', 'background'),
        ('muted', 'panel'),
        ('muted', 'heading'),
        ('on_primary', 'primary'),
        ('on_primary', 'primary_hover'),
        ('on_primary', 'danger'),
        ('on_primary', 'danger_hover'),
    ]
    pairs += [(status + '_text', status + '_background') for status in ('ok', 'warning', 'error', 'pending')]
    for foreground, background in pairs:
        assert ratio(palette[foreground], palette[background]) >= 4.5, (mode, foreground, background)
    assert ratio(palette['focus'], palette['panel']) >= 3


def test_paletas_independientes_y_tema_sistema_resuelto_por_la_app():
    palette = tokens('Claro')
    palette['text'] = '#ffffff'
    assert tokens('Claro')['text'] != palette['text']
    with pytest.raises(ValueError, match='Sistema'):
        tokens('Sistema')
    with pytest.raises(ValueError, match='Rol visual'):
        button_options('desconocido')


def test_asistente_conserva_preferencias_y_estilos_al_cambiar_tema(tmp_path):
    run_gui(
        tmp_path,
        """
from tkinter import ttk
from nurus.personal import ui
from nurus.ui_theme import tokens
secondary=ui.Button(app.pages['Trabajo'],text='Prueba secundaria',role='secondary')
danger=ui.Button(app.pages['Trabajo'],text='Prueba peligro',role='danger')
secondary.pack(side='bottom');app.update()
for choice in ('Claro','Oscuro','Sistema','Claro'):
    app.theme_choice.set(choice);app.apply_appearance();app.update()
    assert app.cfg.data['vista']['tema']==choice
    palette=tokens('Oscuro' if ui.ctk.get_appearance_mode()=='Dark' else 'Claro')
    style=ttk.Style(app)
    assert style.lookup('Primary.TButton','background')==palette['primary']
    assert style.lookup('Danger.TButton','foreground')==palette['on_primary']
    assert style.lookup('OK.TLabel','foreground')==palette['ok_text']
    assert style.lookup('Treeview','background',('selected',))==palette['primary']
    assert style.lookup('Primary.TButton','background',('disabled','active'))==palette['heading']
    assert style.lookup('Secondary.TButton','foreground',('disabled',))==palette['muted']
calls=[]
secondary.configure(command=lambda:calls.append('invocado'))
secondary._canvas.focus_force();app.update()
secondary._canvas.event_generate('<Return>');app.update()
assert calls==['invocado']
assert secondary.cget('border_width')>=2
assert secondary.cget('fg_color')==ui.COLORS['panel']
assert danger.cget('fg_color')==ui.COLORS['danger']
assert app.work_primary_button.cget('text')=='Procesar Excel'
""",
    )


def test_descargador_comparte_tokens_sin_cambiar_preferencias_o_controles(tmp_path):
    root = Path(__file__).resolve().parents[1]
    script = """
from pathlib import Path
import sys, tkinter as tk
from tkinter import ttk
from unittest.mock import patch
from descargador import Application
from test_lotes import dispose_interface
from nurus.ui_theme import tokens
root=tk.Tk();root.withdraw()
app=Application(root,start_worker=False,preferences_path=Path(sys.argv[1])/'preferencias.json')
try:
    for choice,dark in [('Claro',False),('Oscuro',True),('Sistema',True)]:
        app.theme.set(choice)
        with patch('apariencia.system_dark',return_value=dark):app.apply_theme()
        palette=tokens('Oscuro' if dark else 'Claro');style=ttk.Style(root)
        assert app.prefs.data['tema']==choice
        assert style.lookup('Primary.TButton','background')==palette['primary']
        assert style.lookup('Warning.TLabel','background')==palette['warning_background']
        assert str(app.download_button.cget('state'))=='disabled'
        assert app.download_button.cget('text')=='Descargar el lote completo'
        assert app.download_button.cget('style')=='Primary.TButton'
        assert app.package_button.cget('text')=='Crear paquete de este lote…'
        assert app.package_consolidated.get()
    from preferencias import Preferences
    assert Preferences(Path(sys.argv[1])/'preferencias.json').data['tema']=='Sistema'
finally:dispose_interface(app,root)
"""
    result = subprocess.run(
        [sys.executable, '-c', script, str(tmp_path)],
        cwd=root / 'descargador',
        env={**os.environ, 'PYTHONPATH': str(root / 'src')},
        capture_output=True,
        text=True,
        timeout=45,
    )
    assert result.returncode == 0, result.stdout + result.stderr
