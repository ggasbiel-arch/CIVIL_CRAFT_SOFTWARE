"""Projeto 2: automação Python para o formato experimental .bim.json.

Exemplos:
  python python/civil_tools.py validate samples/casa_piloto.bim.json
  python python/civil_tools.py summary samples/casa_piloto.bim.json
  python python/civil_tools.py add-wall samples/casa_piloto.bim.json saida.bim.json --start 0 0 --end 3 0
"""
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

    definitions = project.get("component_definitions", {})
    instances = project.get("component_instances", [])
    if not isinstance(definitions, dict) or not isinstance(instances, list):
        raise ValueError("Catálogo ou instâncias de componentes inválidos")
    for key, definition in definitions.items():
        if not key.strip() or not isinstance(definition, dict) or definition.get("id") != key:
            raise ValueError("ID de definição inválido")
    for instance in instances:
        if not isinstance(instance, dict):
            raise ValueError("Instância de componente inválida")
        ident = instance.get("id")
        if not isinstance(ident, str) or not ident.strip() or ident in ids:
            raise ValueError("ID de instância inválido ou repetido")
        definition_id = instance.get("definition_id")
        level_id = instance.get("level_id")
        if (not isinstance(definition_id, str) or definition_id not in definitions
                or not isinstance(level_id, str) or level_id not in level_ids):
            raise ValueError(f"Referência de componente inválida: {ident}")
        ids.add(ident)


def quantities(project: dict) -> dict:
    output = {"wall_length_m": 0.0, "wall_gross_area_m2": 0.0,
              "wall_gross_volume_m3": 0.0, "slab_area_m2": 0.0, "slab_volume_m3": 0.0}
    for element in project["elements"]:
        if element["type"] == "wall":
            length = math.dist(element["start"], element["end"])
            area = length * element["height_m"]
            output["wall_length_m"] += length
            output["wall_gross_area_m2"] += area
            output["wall_gross_volume_m3"] += area * element["thickness_m"]
        elif element["type"] == "slab":
            area = polygon_area(element["polygon"])
            output["slab_area_m2"] += area
            output["slab_volume_m3"] += area * element["thickness_m"]
    return output


def add_wall(project: dict, level_id: str, start: list[float], end: list[float], thickness_m: float, height_m: float) -> str:
    used = {element["id"] for element in project["elements"]}
    used.update(instance["id"] for instance in project.get("component_instances", []))
    number = 1
    while f"W{number}" in used:
        number += 1
    element_id = f"W{number}"
    project["elements"].append({"type": "wall", "id": element_id, "level_id": level_id,
                                "start": start, "end": end, "thickness_m": thickness_m,
                                "height_m": height_m})
    try:
        validate_project(project)
    except (ValueError, KeyError, TypeError):
        project["elements"].pop()
        raise
    return element_id


def main() -> None:
    parser = argparse.ArgumentParser(description="Ferramentas de projeto — CAD/BIM 0.1")
    subs = parser.add_subparsers(dest="command", required=True)
    for action in ("validate", "summary"):
        command = subs.add_parser(action)
        command.add_argument("file", type=Path)
    create = subs.add_parser("add-wall")
    create.add_argument("file", type=Path)
    create.add_argument("output", type=Path)
    create.add_argument("--level", default="P0")
    create.add_argument("--start", nargs=2, type=float, required=True)
    create.add_argument("--end", nargs=2, type=float, required=True)
    create.add_argument("--thickness", type=float, default=0.2)
    create.add_argument("--height", type=float, default=2.8)
    args = parser.parse_args()
    project = read_project(args.file)
    if args.command == "validate":
        count = len(project["elements"]) + len(project.get("component_instances", []))
        print(f"OK: {project['name']} | {count} elementos")
    elif args.command == "summary":
        print(json.dumps({"name": project["name"], "quantities": quantities(project)}, indent=2, ensure_ascii=False))
    elif args.command == "add-wall":
        ident = add_wall(project, args.level, args.start, args.end, args.thickness, args.height)
        args.output.write_text(json.dumps(project, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"Elemento {ident} criado em {args.output}")


if __name__ == "__main__":
    main()
