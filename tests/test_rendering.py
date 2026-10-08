from nurus.domain.models import Product, ProductKind, ProductStatus, Template
from nurus.services.rendering import prepare, render
from nurus.storage.database import Database


def email_template() -> Template:
    return Template(
        "x", "Correo", ProductKind.EMAIL,
        "Ingreso {TRIBUNAL}", "Hola {PROGRAMA}",
        ("TRIBUNAL", "PROGRAMA"), "published"
    )


def test_missing_recipient_is_warning_not_blocker_for_human_review_draft():
    product = prepare(
        Product(
            ProductKind.EMAIL,
            email_template(),
            {"TRIBUNAL": "Laja", "PROGRAMA": "PRM"},
        )
    )
    assert product.status is ProductStatus.READY
    assert not product.issues
    assert any("Sin destinatario" in warning for warning in product.warnings)


def test_unknown_variable_is_never_silently_consumed():
    template = Template(
        "x", "x", ProductKind.EMAIL,
        "{INEXISTENTE}", "x", ("TRIBUNAL",), "published"
    )
    subject, _, issues = render(template, {"TRIBUNAL": "Laja"})
    assert subject == "{INEXISTENTE}"
    assert issues == ["Variable no permitida: INEXISTENTE"]


def test_malformed_placeholder_is_blocking():
    template = Template(
        "x", "x", ProductKind.EMAIL,
        "Ingreso {TRIBUNAL", "Hola", ("TRIBUNAL",), "published"
    )
    product = prepare(Product(ProductKind.EMAIL, template, {"TRIBUNAL": "Laja"}))
    assert product.status is ProductStatus.BLOCKED
    assert "Sintaxis de variable inválida en la plantilla." in product.issues


def test_placeholder_literal_does_not_satisfy_required_business_data():
    template = Template(
        "x", "x", ProductKind.EMAIL,
        "Ingreso {TRIBUNAL}", "{PROGRAMA}",
        ("TRIBUNAL", "PROGRAMA"), "published"
    )
    product = prepare(
        Product(
            ProductKind.EMAIL,
            template,
            {"TRIBUNAL": "Laja", "PROGRAMA": "por completar"},
        )
    )
    assert product.status is ProductStatus.BLOCKED
    assert "Falta dato para: PROGRAMA" in product.issues


def test_product_with_recipient_and_data_is_ready():
    product = prepare(
        Product(
            ProductKind.EMAIL,
            email_template(),
            {"TRIBUNAL": "Jgdo. L. y G. de Laja", "PROGRAMA": "PRM Alfa"},
            recipient="prm@example.invalid",
        )
    )
    assert product.status is ProductStatus.READY
    assert "Jgdo. L. y G. de Laja" in product.rendered_subject


def test_history_keeps_the_reviewed_snapshot(tmp_path):
    db = Database(tmp_path / "nurus.sqlite3")
    product = prepare(
        Product(
            ProductKind.EMAIL,
            email_template(),
            {"TRIBUNAL": "Laja", "PROGRAMA": "PRM"},
            recipient="prm@example.invalid",
        )
    )
    db.record_product(product)
    saved = db.list_products()
    assert len(saved) == 1
    assert saved[0]["subject"] == "Ingreso Laja"

