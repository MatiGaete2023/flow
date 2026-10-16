"""Motor de texto: patrones de identificadores, seudónimos estables y revisión final.

Todo es local y determinista dentro de una sesión: el mismo dato recibe siempre el mismo reemplazo,
así se conservan hermanos, causas y relaciones entre archivos procesados juntos.
"""

from __future__ import annotations

import hashlib
import hmac
import re
import secrets
import unicodedata
from dataclasses import dataclass, field
from functools import lru_cache
from datetime import date, datetime, timedelta

OMITTED = "[texto omitido]"
IDENTITY_OMITTED = "[identidad omitida]"
FAKE_DOMAIN = "@example.invalid"
LINK_OMITTED = "[enlace omitido]"

DASH = "\\-\u2010\u2011\u2012\u2013\u2014\u2212"
RUT_RE = re.compile(
    r"(?<![\w.])(\d{1,2}(?:\.\d{3}){2}|\d{1,2}(?: \d{3}){2}|\d{6,8})(\s*[" + DASH + r"]\s*|\s+)([\dkK])(?![\w])"
)
RUT_NO_DV_RE = re.compile(r"(?<![\w.])\d{1,2}(?:\.\d{3}){2}(?![\w.])")
RIT_STRICT_RE = re.compile(r"(?<!\w)([A-Za-z])[" + DASH + r"](\d{1,6})[" + DASH + r"/]((?:19|20)\d{2})(?!\w)")
RIT_FREE_RE = re.compile(
    r"(?<!\w)(?:([A-Za-z])[\s" + DASH + r"]+)?(\d{1,6})[\s" + DASH + r"/]+((?:19|20)\d{2})(?![\w])"
)
PHONE_RE = re.compile(
    r"(?<![\w+])(?:\+?56[\s-]?)?(?:9[\s-]?\d{4}[\s-]?\d{4}|\d{3}[\s-]\d{3}[\s-]\d{3}"
    r"|\d{2}[\s-]?\d{3}[\s-]?\d{4}|\d{2}[\s-]?\d{7})(?!\d)"
)
GROUPED_DIGITS_RE = re.compile(r"(?<![\w.])\d(?:[\s" + DASH + r"]?\d){7,}(?![\w])")
DIGITS_RE = re.compile(r"(?<![\d.\-])\d{7,12}(?![\d\-])")
MAIL_RE = re.compile(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+")
URL_RE = re.compile(r"(?i)\b(?:https?://|ftp://|www\.)[^\s<>\"')\]]+")
ADDRESS_RE = re.compile(
    r"(?i)\b(?:av(?:enida)?|calle|pasaje|pje|villa|pobl(?:aci[oó]n)?|block|depto|camino)\.?\s+"
    r"[\wÁÉÍÓÚÑáéíóúñ .'-]{2,40}?\d{1,5}"
)
PATH_RE = re.compile(
    r"(?i)(?:\b[A-Z]:[\\/]+(?:Users|Usuarios|Documents and Settings)[\\/]+|(?<![\w.])/(?:home|Users)/)"
    r"[^\\/:*?\"<>|\r\n]+"
)
UNC_RE = re.compile(r"\\\\[^\\/\s]+(?=\\)")
DATE_ISO_RE = re.compile(r"(?<!\d)(\d{4})-(\d{2})-(\d{2})(?!\d)")
DATE_DMY_RE = re.compile(r"(?<!\d)(\d{1,2})([/.\-])(\d{1,2})\2(\d{4})(?!\d)")

CAP = r"[A-ZÁÉÍÓÚÑÜ][a-záéíóúñü]{2,}"
UP = r"[A-ZÁÉÍÓÚÑÜ]{3,}"
PART = r"(?:(?:de|del|la|las|los|san|santa|y)\s+)"
UNKNOWN_FULL_NAME = re.compile(r"(?<!\w)" + CAP + r"(?:\s+" + PART + r"?" + CAP + r"){1,3}(?!\w)")
UNKNOWN_UPPER_NAME = re.compile(r"(?<!\w)" + UP + r"(?:\s+" + PART + r"?" + UP + r"){1,3}(?!\w)")
ROLES = (
    "sr|sra|srta|don|doña|dona|dr|dra|nna|madre|padre|mamá|mama|papá|papa|abuela|abuelo|tía|tia|tío|tio|"
    "hermano|hermana|hijo|hija|curador|curadora|abogado|abogada|abog|psicólogo|psicologo|psicóloga|psicologa|"
    "contactar a|hablar con|llamar a|llamó a|llamo a|apoderado|apoderada|tutor|tutora|menor|joven|niño|nino|niña|nina"
)
AFTER_ROLE = re.compile(
    r"((?i:\b(?:"
    + ROLES
    + r")\.?\s+))((?:"
    + CAP
    + "|"
    + UP
    + r")(?:\s+"
    + PART
    + r"?(?:"
    + CAP
    + "|"
    + UP
    + r")){0,3})"
)
INSTITUTIONAL = {
    "tribunal",
    "juzgado",
    "familia",
    "corte",
    "apelaciones",
    "suprema",
    "programa",
    "centro",
    "residencial",
    "ambulatorio",
    "fae",
    "dce",
    "prm",
    "ppf",
    "pie",
    "pro",
    "senam",
    "mejor",
    "ninez",
    "sename",
    "servicio",
    "proteccion",
    "curaduria",
    "ministerio",
    "publico",
    "fiscalia",
    "defensoria",
    "penal",
    "civil",
    "laboral",
    "garantia",
    "oral",
    "letras",
    "cobranza",
    "audiencia",
    "preparatoria",
    "juicio",
    "medida",
    "cautelar",
    "oficina",
    "unidad",
    "region",
    "comuna",
    "municipalidad",
    "hospital",
    "consultorio",
    "colegio",
    "escuela",
    "liceo",
    "jardin",
    "infantil",
    "carabineros",
    "chile",
    "pdi",
    "gendarmeria",
    "registro",
    "rus",
    "sitfa",
    "persona",
    "ficticia",
    "nurus",
    "csmp",
    "poder",
    "judicial",
    "ucc",
    "concepcion",
    "laja",
    "mulchen",
    "tome",
    "santiago",
}
PLACEHOLDERS = {
    "",
    "no",
    "no posee",
    "no tiene",
    "sin designar",
    "sin curador",
    "sin rut",
    "n a",
    "na",
    "no aplica",
    "s i",
    "si",
    "sin informacion",
    "sin dato",
    "sd",
    "pendiente",
}
STOPWORDS = {
    "de",
    "del",
    "la",
    "las",
    "los",
    "san",
    "santa",
    "y",
    "e",
    "van",
    "von",
    "da",
    "dos",
    "abog",
    "sr",
    "sra",
    "srta",
    "don",
    "dona",
    "fono",
    "rut",
    "run",
}
SAFE_STRUCT = re.compile(
    r"[+-]?\d{1,6}(?:[.,]\d+)?"
    r"|\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?"
    r"|\d{1,2}[/.-]\d{1,2}[/.-]\d{4}(?:[ T]\d{1,2}:\d{2}(?::\d{2})?)?"
    r"|(?i:true|false|si|sí|no)"
)
_FOLD = {"a": "aáàäâ", "e": "eéèëê", "i": "iíìïî", "o": "oóòöô", "u": "uúùüû", "n": "nñ", "c": "cç"}
_MARK = re.compile(r"[\uE000\uE001]")
END = "\0"
DIGIT_RE = re.compile(r"\d")
WORD_RE = re.compile(r"[^\W\d_]+")
GAP_RE = re.compile(r"[\s\-.,]+")
HEX_TOKEN_RE = re.compile(r"\b(?:anon-)?[0-9a-f]{24,64}\b")
FAKE_NAME_RE = re.compile(r"Persona Ficticia \d+|(?:Nombre|Apellido)\d+")


def nfc(value):
    return unicodedata.normalize("NFC", value) if isinstance(value, str) else value


@lru_cache(maxsize=1 << 17)
def _fold(value):
    text = unicodedata.normalize("NFKD", value).encode("ascii", "ignore").decode().lower()
    return re.sub(r"[^a-z0-9]+", " ", text).strip()


def fold(value):
    """Minúsculas, sin tildes ni signos: clave estable para comparar nombres."""
    return _fold(str(value or ""))


def normalize(value):
    text = unicodedata.normalize("NFKD", str(value or "")).encode("ascii", "ignore").decode()
    return re.sub(r"\s+", " ", re.sub(r"[-().:_/°º]+", " ", text.lower())).strip()


def check_digit(number):
    total, factor = 0, 2
    for char in reversed(str(number)):
        total += int(char) * factor
        factor = 2 if factor == 7 else factor + 1
    rest = 11 - total % 11
    return {11: "0", 10: "K"}.get(rest, str(rest))


def is_placeholder(text):
    norm = normalize(text)
    return norm in PLACEHOLDERS or not re.search(r"[a-z0-9]", norm)


def folded_pattern(text):
    parts = []
    for char in fold(text):
        if char == " ":
            parts.append(r"[\s\-.,]+")
        elif char in _FOLD:
            parts.append("[" + re.escape(_FOLD[char] + _FOLD[char].upper()) + "]")
        else:
            parts.append(re.escape(char))
    return "".join(parts)


def curator_name(text):
    """Nombre de un curador individual; vacío para placeholders y curadores institucionales."""
    if is_placeholder(text):
        return ""
    if "Institución:" in text:
        match = re.search(r"(?i)\b(?:abog(?:ad[oa])?|sr|sra|srta)\.?\s+([^()\[\],;:0-9]+)", text)
        return curator_name(match.group(1)) if match else ""
    rest = RUT_RE.sub(" ", text)
    rest = PHONE_RE.sub(" ", rest)
    rest = MAIL_RE.sub(" ", rest)
    rest = re.sub(r"(?i)\b(rut|run|fono|tel[eé]fono|abog|abogad[oa]|sr|sra|srta|curador[a]?)\b\.?", " ", rest)
    rest = re.sub(r"[()\[\]:;,/|0-9]", " ", rest)
    return re.sub(r"\s+", " ", rest).strip(" -.")


def shift_dates(text, days):
    """Desplaza fechas escritas en texto conservando el formato (ISO o dd-mm-aaaa)."""
    if not days or not isinstance(text, str):
        return text

    def iso(match):
        try:
            moved = date(int(match.group(1)), int(match.group(2)), int(match.group(3))) + timedelta(days=days)
        except (ValueError, OverflowError):
            return match.group(0)
        return f"{moved.year:04d}-{moved.month:02d}-{moved.day:02d}"

    def dmy(match):
        try:
            moved = date(int(match.group(4)), int(match.group(3)), int(match.group(1))) + timedelta(days=days)
        except (ValueError, OverflowError):
            return match.group(0)
        sep, wide = match.group(2), len(match.group(1)) == 2 or len(match.group(3)) == 2
        day, month = (f"{moved.day:02d}", f"{moved.month:02d}") if wide else (str(moved.day), str(moved.month))
        return f"{day}{sep}{month}{sep}{moved.year:04d}"

    return DATE_DMY_RE.sub(dmy, DATE_ISO_RE.sub(iso, text))


def shift_value(value, days):
    if days and isinstance(value, (datetime, date)) and not isinstance(value, bool):
        try:
            return value + timedelta(days=days)
        except OverflowError:
            return value
    return value


@dataclass
class Options:
    mode: str = "estricto"
    shift_days: int = 0
    seed: str = ""
    skip_unsupported: bool = True
    generalize_birth: bool = False

    def __post_init__(self):
        if self.mode not in ("estricto", "conservador"):
            raise ValueError("Modo de anonimización desconocido.")


class Stash:
    """Guarda reemplazos ya hechos para que ningún patrón posterior los vuelva a tocar."""

    def __init__(self):
        self.items = []

    def put(self, text):
        self.items.append(text)
        return f"\ue000{len(self.items) - 1}\ue001"

    def restore(self, text):
        return re.sub(r"\uE000(\d+)\uE001", lambda m: self.items[int(m.group(1))], text)


class Pseudonyms:
    def __init__(self, seed="", reserved_rits=()):
        self.seed = seed.encode() if seed else b""
        self.secret = self.seed or secrets.token_bytes(32)
        self.values = {}
        self.counters = {}
        self.used = set()
        self.reserved_rits = set(reserved_rits)

    def _number(self, kind, key, modulus):
        if not self.seed:
            self.counters[kind] = self.counters.get(kind, 0) + 1
            return self.counters[kind]
        digest = hmac.new(self.seed, f"{kind}|{key}".encode(), hashlib.sha256).digest()
        number = int.from_bytes(digest[:6], "big") % modulus + 1
        while (kind, number) in self.used:
            number = number % modulus + 1
        self.used.add((kind, number))
        return number

    @staticmethod
    def rit_key(text):
        match = RIT_FREE_RE.search(text)
        if match:
            return f"{(match.group(1) or '').upper()}-{int(match.group(2))}-{match.group(3)}"
        return text.upper()

    def get(self, kind, original):
        original = str(original).strip()
        if kind in ("persona", "direccion"):
            canon = fold(original)
        elif kind == "rit":
            canon = self.rit_key(original)
        elif kind == "rut":
            canon = re.sub(r"[^\dkK]", "", original).upper()
        else:
            canon = original.upper()
        key = (kind, canon)
        if key in self.values:
            return self.values[key]
        if kind == "rut":
            number = self._number(kind, canon, 9_000_000)
            base = 90_000_000 + number
            value = f"{base}-{'1' if check_digit(base) != '1' else '2'}"
        elif kind == "rit":
            match = RIT_FREE_RE.search(original)
            letter = (match.group(1) or "X").upper() if match else "X"
            year = match.group(3) if match else "2000"
            number = self._number(kind, canon, 999_999)
            while f"{letter}-{number}-{year}" in self.reserved_rits:
                number = number % 999_999 + 1
            if not self.seed:
                self.counters[kind] = number
            value = f"{letter}-{number}-{year}"
        elif kind == "mail":
            value = f"contacto{self._number(kind, canon, 10**6)}{FAKE_DOMAIN}"
        elif kind == "telefono":
            value = f"9{self._number(kind, canon, 10**8 - 1):08d}"
        elif kind == "direccion":
            value = f"Calle Ficticia {self._number(kind, canon, 10**6)}"
        elif kind == "numero":
            value = f"ID-{self._number(kind, canon, 10**8)}"
        elif kind == "hash":
            value = hmac.new(self.secret, canon.encode(), hashlib.sha256).hexdigest()
        elif kind.startswith("tok:"):
            value = f"{kind[4:][:3]}_anon_{self._number(kind, canon, 10**8)}"
        else:
            value = f"Persona Ficticia {self._number(kind, canon, 10**6)}"
        self.values[key] = value
        return value

    def fakes(self, kind):
        return {value for (k, _), value in self.values.items() if k == kind}


def fake_rut(pseudonyms, match):
    """RUT ficticio con el mismo formato (puntos y separador) que el original."""
    digits, separator, dv = match.group(1), match.group(2), match.group(3)
    fake = pseudonyms.get("rut", re.sub(r"\D", "", digits) + "-" + dv.upper())
    number, _, fake_dv = fake.partition("-")
    thousands = "." if "." in digits else " " if " " in digits else ""
    if thousands:
        number = f"{int(number):,}".replace(",", thousands)
    return number + separator + fake_dv


class Replacer:
    def __init__(self, pseudonyms, options):
        self.pseudonyms = pseudonyms
        self.options = options
        self.names = {}
        self.tokens = {}
        self._compiled = None

    def add_person(self, full):
        full = re.sub(r"\s+", " ", str(full)).strip()
        if len(full) < 3 or is_placeholder(full) or not re.search(r"[^\W\d_]", full):
            return
        fake = self.pseudonyms.get("persona", full)
        self.names.setdefault(full, fake)
        words = [w.strip(",.;:()") for w in full.split()]
        for index, token in enumerate(words):
            if len(token) >= 3 and normalize(token) not in STOPWORDS and re.search(r"[^\W\d_]", token):
                number = fake.rsplit(" ", 1)[-1]
                self.tokens.setdefault(token, ("Apellido" if index else "Nombre") + number)
        self._compiled = None

    def _index(self, full_only=False):
        """Árbol de palabras plegadas -> falso, de nombres completos y, salvo `full_only`, de cada palabra."""
        if self._compiled is None:
            self._compiled = {}
        if full_only not in self._compiled:
            items = dict(self.names) if full_only else {**self.tokens, **self.names}
            root = {}
            for original, fake in items.items():
                node = root
                words = fold(original).split()
                if not words:
                    continue
                for word in words:
                    node = node.setdefault(word, {})
                node.setdefault(END, fake)
            self._compiled[full_only] = root
        return self._compiled[full_only]

    def _matches(self, text, full_only=False):
        """[(inicio, fin, falso, clave)] de nombres conocidos, de izquierda a derecha y sin solaparse."""
        root = self._index(full_only)
        if not root:
            return []
        spans = [(m.start(), m.end(), fold(m.group())) for m in WORD_RE.finditer(text)]
        if not any(word in root for _, _, word in spans):
            return []
        found, i = [], 0
        while i < len(spans):
            node = root.get(spans[i][2])
            if node is None:
                i += 1
                continue
            ends, j = [], i
            if END in node:
                ends.append((j, node[END]))
            while j + 1 < len(spans) and GAP_RE.fullmatch(text[spans[j][1] : spans[j + 1][0]]):
                node = node.get(spans[j + 1][2])
                if node is None:
                    break
                j += 1
                if END in node:
                    ends.append((j, node[END]))
            hit = None
            for last, fake in reversed(ends):
                start, stop = spans[i][0], spans[last][1]
                before = text[start - 1] if start else " "
                after = text[stop] if stop < len(text) else " "
                if not (before.isalnum() or before == "_" or after.isalnum() or after == "_"):
                    hit = (start, stop, fake, " ".join(w for _, _, w in spans[i : last + 1]), last)
                    break
            if hit:
                found.append(hit[:4])
                i = hit[4] + 1
            else:
                i += 1
        return found

    def known_names(self, text, full_only=False):
        """Coincidencias de nombres conocidos en `text` (lista de claves plegadas)."""
        return [key for _, _, _, key in self._matches(text, full_only)]

    def _names(self, text, stash, full_only=False):
        out, last = [], 0
        for start, end, fake, _ in self._matches(text, full_only):
            out.append(text[last:start])
            out.append(stash.put(fake))
            last = end
        out.append(text[last:])
        return "".join(out)

    def identifiers(self, text, *, free, phones=False, stash=None):
        stash = stash if stash is not None else Stash()
        text = _MARK.sub("", text) if ("\ue000" in text or "\ue001" in text) else text
        pseudo = self.pseudonyms
        days = self.options.shift_days
        digits = DIGIT_RE.search(text) is not None
        if digits and (free or days):
            text = DATE_ISO_RE.sub(lambda m: stash.put(shift_dates(m.group(0), days)), text)
            text = DATE_DMY_RE.sub(lambda m: stash.put(shift_dates(m.group(0), days)), text)
        if free:
            if "/" in text or "." in text:
                text = URL_RE.sub(lambda m: stash.put(LINK_OMITTED), text)
            if "\\" in text or "/" in text:
                text = PATH_RE.sub(lambda m: stash.put(_path_prefix(m.group(0)) + "usuario"), text)
                text = UNC_RE.sub(lambda m: stash.put("\\\\servidor"), text)
        if "@" in text:
            text = MAIL_RE.sub(
                lambda m: m.group(0) if m.group(0).endswith(FAKE_DOMAIN) else stash.put(pseudo.get("mail", m.group(0))),
                text,
            )
        if digits:
            text = RUT_RE.sub(lambda m: stash.put(fake_rut(pseudo, m)), text)
            text = RUT_NO_DV_RE.sub(lambda m: stash.put("#" * len(m.group(0))), text)
            text = (RIT_FREE_RE if free else RIT_STRICT_RE).sub(lambda m: self._rit(m, stash), text)
            if free or phones:
                text = PHONE_RE.sub(lambda m: stash.put(pseudo.get("telefono", re.sub(r"\D", "", m.group(0)))), text)
            if free:
                text = ADDRESS_RE.sub(lambda m: stash.put(pseudo.get("direccion", m.group(0))), text)
                text = GROUPED_DIGITS_RE.sub(lambda m: stash.put("#" * len(re.sub(r"\s", "", m.group(0)))), text)
                text = DIGITS_RE.sub(lambda m: stash.put("#" * len(m.group(0))), text)
        return text

    def _rit(self, match, stash):
        if match.re is RIT_FREE_RE:
            letter, text = match.group(1), match.group(0)
            if not letter and int(match.group(2)) <= 12:
                return text  # «10/2026» es mes y año, no una causa
            if letter and letter.islower() and text[1:2].isspace():
                return text  # «y 10/2026»: la «y» es una conjunción, no el tipo de causa
        return stash.put(self.pseudonyms.get("rit", match.group(0)))

    def text(self, value, *, free=True, phones=False, scrub=False, full_only=False):
        """Reemplaza identificadores y nombres conocidos. Con `scrub` también candidatos no catalogados.

        `full_only` (columnas que lee el motor) reemplaza nombres completos pero no palabras sueltas, para no
        alterar programas o tribunales cuyo texto coincide con un apellido."""
        stash = Stash()
        text = self.identifiers(value, free=free, phones=phones, stash=stash)
        text = self._names(text, stash, full_only)
        if scrub:
            text = scrub_unknown(text)
        return stash.restore(text)


def _path_prefix(path):
    for marker in ("\\", "/"):
        if marker in path:
            head = path.rsplit(marker, 1)[0]
            return head + marker
    return ""


def _institutional(text):
    words = [w for w in fold(text).split() if w not in STOPWORDS]
    return bool(words) and all(w in INSTITUTIONAL for w in words)


def scrub_unknown(text):
    """Oculta secuencias que parecen nombres de persona aunque no estén catalogadas."""
    text = AFTER_ROLE.sub(lambda m: m.group(1) + (m.group(2) if _institutional(m.group(2)) else IDENTITY_OMITTED), text)
    for pattern in (UNKNOWN_FULL_NAME, UNKNOWN_UPPER_NAME):
        text = pattern.sub(lambda m: m.group(0) if _institutional(m.group(0)) else IDENTITY_OMITTED, text)
    return text


@dataclass
class Report:
    files: list = field(default_factory=list)
    warnings: list = field(default_factory=list)

    def warn(self, text):
        if text not in self.warnings:
            self.warnings.append(text)


class Session:
    """Estado compartido por todos los archivos de una misma ejecución."""

    def __init__(self, options=None):
        self.options = options or Options()
        self.pseudonyms = Pseudonyms(self.options.seed)
        self.replacer = Replacer(self.pseudonyms, self.options)
        self.report = Report()
        self.name_map = {}

    def token(self, kind, value):
        """Identificador técnico estable (causa, ingreso, huella, archivo…): une tablas distintas entre sí."""
        if value in ("", None):
            return ""
        key = str(value)
        if kind in ("source_sha256", "sha256"):
            return self.pseudonyms.get("hash", kind + ":" + key)
        fake = self.pseudonyms.get("tok:" + kind, key)
        number = fake.rsplit("_", 1)[1]
        if kind == "original_file":
            ext = "." + key.rsplit(".", 1)[-1].lower() if "." in key else ""
            return f"fuente_{number}{ext if ext in ('.xls', '.xlsx', '.html', '.htm', '.csv') else '.xls'}"
        if kind == "original_sheet":
            return f"Hoja{number}"
        return fake

    @property
    def strict(self):
        return self.options.mode == "estricto"

    def reserve_rits_in(self, text):
        for match in RIT_FREE_RE.finditer(text):
            if (
                match.group(1)
                and not (match.group(1).islower() and match.group(0)[1:2].isspace())
                or (not match.group(1) and int(match.group(2)) > 12)
            ):
                self.pseudonyms.reserved_rits.add(
                    f"{(match.group(1) or 'X').upper()}-{int(match.group(2))}-{match.group(3)}"
                )

    def json_text(self, value):
        """Celda o campo que contiene JSON: se anonimiza como estructura; si no es JSON, como texto libre."""
        from . import jsonfmt

        return jsonfmt.text(value, self)

    def seed_json_text(self, value):
        from . import jsonfmt

        jsonfmt.seed_text(value, self)

    def free(self, value):
        """Texto libre según el modo: omitido (estricto) o depurado (conservador)."""
        if not isinstance(value, str) or not value.strip():
            return value
        if self.strict:
            return OMITTED
        return self.replacer.text(value, free=True, scrub=True)

    def leaks(self, texts):
        """Revisión final de lo ya construido: (categorías de filtración, residuos permitidos).

        Nunca devuelve los datos. Una palabra de nombre dentro de una celda que lee el motor (`protected`)
        se cuenta como residuo y no como filtración; un nombre completo siempre es filtración."""
        found, residual = set(), 0
        pseudo = self.pseudonyms
        fake_ruts = {f.split("-")[0] for f in pseudo.fakes("rut")}
        fake_phones = pseudo.fakes("telefono")
        fake_rits = pseudo.fakes("rit")
        full_keys = {fold(n) for n in self.replacer.names}
        for text, protected in texts:
            text = nfc(text)
            if not isinstance(text, str) or not text:
                continue
            text = HEX_TOKEN_RE.sub(" ", FAKE_NAME_RE.sub(" ", text))  # huellas e ids aleatorios no son datos
            for match in RIT_STRICT_RE.finditer(text):
                if f"{match.group(1).upper()}-{int(match.group(2))}-{match.group(3)}" not in fake_rits:
                    found.add("RIT")
            text = RIT_STRICT_RE.sub(" ", text)  # los RIT ficticios de 5 dígitos parecen teléfonos
            for match in RUT_RE.finditer(text):
                digits = re.sub(r"\D", "", match.group(1))
                if digits not in fake_ruts and check_digit(digits) == match.group(3).upper():
                    found.add("RUT con dígito válido")
            if any(not m.group(0).endswith(FAKE_DOMAIN) for m in MAIL_RE.finditer(text)):
                found.add("correo")
            if any(re.sub(r"\D", "", m.group(0)) not in fake_phones for m in PHONE_RE.finditer(text)):
                found.add("teléfono")
            if PATH_RE.search(text) and not re.search(r"(?i)[\\/]usuario(?:[\\/]|$)", text):
                found.add("ruta con usuario")
            for name in self.replacer.known_names(text):
                key = fold(name)
                if key in full_keys or len(key) >= 4 and not protected:
                    found.add("nombre")
                elif len(key) >= 4:
                    residual += 1
        return found, residual
