"""Acceso anterior conservado: abre únicamente CSMP Assistant personal.

La interfaz NuRus retirada permanece recuperable en el historial Git.
"""


def main() -> None:
    from nurus.personal.app import main as personal_main
    personal_main()


if __name__ == "__main__":
    main()
