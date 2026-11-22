"""Consulta de arquivos IFC STEP pelo aplicativo desktop."""
import sys
from collections import Counter
from pathlib import Path


def inspect(path):
    import ifcopenshell

    if path.suffix.lower() != ".ifc" or not path.is_file():
        raise ValueError("Informe o caminho de um arquivo .ifc existente.")
    model = ifcopenshell.open(str(path))
    elements = model.by_type("IfcElement")
    counts = Counter(element.is_a() for element in elements)
    lines = [f"Arquivo: {path.name}", f"Esquema: {model.schema_identifier}",
             f"Elementos: {len(elements)}"]
    lines.extend(f"{kind}: {count}" for kind, count in sorted(counts.items()))
    if elements:
        lines.extend(["", f"Primeiros {min(100, len(elements))} elementos (classe, nome e GlobalId)"])
    for element in elements[:100]:
        name = " ".join((element.Name or "Sem nome").split())[:200]
        lines.append(f"{element.is_a()}  ·  {name}  ·  {element.GlobalId}")
    return "\n".join(lines)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
    try:
        print(inspect(Path(sys.argv[1])))
    except ImportError as error:
        sys.exit(f"Instale as dependências IFC no Python usado pelo aplicativo: {error}")
    except Exception as error:
        sys.exit(f"Falha ao ler IFC: {error}")
