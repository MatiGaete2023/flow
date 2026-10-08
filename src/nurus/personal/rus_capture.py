"""Inspección de capturas actuales. No es un lector completo ni un escritor RUS."""
import argparse
from datetime import datetime
from hashlib import sha256
import json
from pathlib import Path
from urllib.parse import urlsplit


def inspect_capture(path):
    raw = Path(path).read_bytes()
    if len(raw) > 20 * 1024 * 1024:
        raise ValueError('La captura supera 20 MB; captura solamente la ventana del registro.')
    data = json.loads(raw)
    if not isinstance(data, dict) or data.get('schema') != 'rus-capture-v1':
        raise ValueError('El archivo no es una captura del inspector RUS.')
    if data.get('stage') not in ('bitacora', 'formulario', 'resultado'):
        raise ValueError('La captura no identifica la pantalla.')
    documents = data.get('documents')
    if not isinstance(documents, list) or not documents:
        raise ValueError('La captura no contiene documentos.')
    encoded = json.dumps(documents, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
    if sha256(encoded).hexdigest() != data.get('documents_sha256'):
        raise ValueError('El contenido de la captura no coincide con su huella.')
    moment = datetime.fromisoformat(data['captured_at'].replace('Z', '+00:00'))
    if moment.tzinfo is None:
        raise ValueError('La captura necesita fecha y zona horaria.')
    forms = []
    tables = 0
    for frame in documents:
        doc = frame['document']
        url = urlsplit(doc['url'])
        if (doc.get('schema') != 'rus-document-v1' or url.scheme != 'https'
                or url.netloc != 'familia.pjud.cl' or url.query or url.fragment):
            raise ValueError('Documento ajeno al origen institucional o URL sin depurar.')
        if doc.get('capabilities') != {'writes': False, 'complete_entries': False, 'server_time_verified': False}:
            raise ValueError('La captura declara capacidades que este inspector no acredita.')
        tables += len(doc['tables'])
        for form in doc['forms']:
            controls = form['controls']
            forms.append({
                'frame_id': frame['frame_id'], 'index': form['index'], 'name': form['name'],
                'id': form['id'], 'action': form['action'], 'method': form['method'],
                'text_fields': [{'name': c['name'], 'id': c['id'], 'max_length': c['max_length'],
                                 'visible': c['visible'], 'disabled': c['disabled']}
                                for c in controls if c['tag'] == 'textarea'],
                'selectors': [{'name': c['name'], 'id': c['id'], 'options': c.get('options', [])}
                              for c in controls if c['tag'] == 'select'],
                'buttons': [{'name': c['name'], 'id': c['id'], 'label': c['value'] or c.get('text', '')}
                            for c in controls if c['type'] in ('submit', 'button')],
                'hidden_identity_fields': [c['name'] for c in controls
                                           if c['type'] == 'hidden' and c['redacted'] is False],
            })
    return {
        'schema': 'rus-inspection-v1', 'capture_sha256': sha256(raw).hexdigest(),
        'stage': data['stage'], 'captured_at': data['captured_at'], 'documents': len(documents),
        'forms': forms, 'tables': tables, 'registration_enabled': False,
        'next_checks': [
            'Relacionar campos reales con tribunal, causa, ingreso, persona, centro, etapa y modalidad.',
            'Comprobar identidad del usuario y opciones efectivas de tipo, estado y destino.',
            'Comprobar límite del texto y reglas adicionales del guardado.',
            'Identificar entrada persistida, fecha del servidor y fin de paginación del día actual.',
            'Implementar lector y escritor con estas evidencias y verificar un registro controlado.'
        ],
        'limitations': 'La huella detecta cambios, no autentica el origen. No acredita identidad, cobertura ni guardado.'
    }


def main(argv=None):
    parser = argparse.ArgumentParser(description='Inspeccionar una captura del formulario actual RUS, sin registrar.')
    parser.add_argument('captura', type=Path)
    parser.add_argument('--salida', type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        report = inspect_capture(args.captura)
        # Una inspección nueva no sobrescribe una captura ni otro informe.
        with args.salida.open('x', encoding='utf-8') as handle:
            json.dump(report, handle, ensure_ascii=False, indent=2)
    except (OSError, ValueError, KeyError, TypeError) as exc:
        parser.exit(1, f'No se generó la inspección: {exc}\n')
    print(f'Inspección creada: {args.salida}. Registro RUS todavía no habilitado.')


if __name__ == '__main__':
    main()
