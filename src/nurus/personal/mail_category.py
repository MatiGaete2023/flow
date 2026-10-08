"""Comportamiento operativo independiente del nombre y clave de la plantilla."""
CATEGORIES = ('espera', 'cumplimiento', 'informes', 'medidas', 'proyectos',
              'programa_espera', 'programa_vencido', 'programa_por_vencer')


def category(template, key):
    return template.get('categoria',key) or 'general'
