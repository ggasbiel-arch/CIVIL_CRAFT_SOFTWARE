"""Validation and inspection of .bim.json files; no temporary drawing tools."""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path


def read_project(path: Path) -> dict:
    with path.open(encoding="utf-8") as stream:
        project = json.load(stream)
    validate_project(project)
    return project


def valid_point(point: object) -> bool:
    return (isinstance(point, list) and len(point) == 2 and all(
        isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) for v in point
    ))


def polygon_area(poly: list[list[float]]) -> float:
    return abs(sum(
        poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
        for i in range(len(poly))
    )) / 2 if len(poly) >= 3 else 0.0


def validate_project(project: dict) -> None:
    if project.get("schema_version") != 1:
        raise ValueError("Versão do schema não suportada")
    if not isinstance(project.get("name"), str) or not project["name"].strip():
        raise ValueError("Nome de projeto inválido")
    levels = project.get("levels")
    elements = project.get("elements")
    if not isinstance(levels, list) or not levels or not isinstance(elements, list):
        raise ValueError("Pavimentos ou elementos ausentes")
    ids = set()
    for level in levels:
        lid = level["id"]
        if not isinstance(lid, str) or not lid or lid in ids or not isinstance(level.get("name"), str) or not level["name"].strip():
            raise ValueError("Pavimento inválido ou repetido")
        if not isinstance(level.get("elevation_m"), (int, float)) or not math.isfinite(level["elevation_m"]):
            raise ValueError("Cota de pavimento inválida")
        ids.add(lid)
    level_ids = ids.copy()
    ids.clear()
    for element in elements:
        eid = element["id"]
        if not isinstance(eid, str) or not eid or eid in ids or element.get("level_id") not in level_ids:
            raise ValueError("Elemento com ID duplicado ou pavimento inválido")
        ids.add(eid)
        kind = element.get("type")
        if kind == "wall":
            if not valid_point(element.get("start")) or not valid_point(element.get("end")):
                raise ValueError(f"Coordenadas da parede {eid} inválidas")
            if math.dist(element["start"], element["end"]) < 0.01:
                raise ValueError(f"Parede {eid} sem comprimento")
            dims = (element.get("thickness_m"), element.get("height_m"))
        elif kind == "slab":
            poly = element.get("polygon")
            if not isinstance(poly, list) or len(poly) < 3 or not all(valid_point(p) for p in poly) or polygon_area(poly) < 0.0001:
                raise ValueError(f"Polígono da laje {eid} inválido")
            dims = (element.get("thickness_m"),)
        else:
            raise ValueError(f"Tipo de elemento não suportado: {kind}")
        if not all(isinstance(v, (int, float)) and not isinstance(v, bool) and math.isfinite(v) and v > 0 for v in dims):
            raise ValueError(f"Dimensões de {eid} inválidas")



def project_summary(project: dict) -> dict:
    validate_project(project)
    return {
        "name": project["name"],
        "levels": len(project["levels"]),
        "elements": len(project["elements"]),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Inspeção de projetos CAD/BIM")
    subs = parser.add_subparsers(dest="command", required=True)
    for action in ("validate", "summary"):
        command = subs.add_parser(action)
        command.add_argument("file", type=Path)
    args = parser.parse_args()
    project = read_project(args.file)
    if args.command == "validate":
        print(f"OK: {project['name']} | {len(project['elements'])} elementos")
    elif args.command == "summary":
        print(json.dumps(project_summary(project), indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
