"""Projeto 2: inspecionar/testar o esquema experimental de componentes BIM.

O núcleo de produção deve ser o avaliador Rust (civil_core::components).
Este módulo Python é uma ferramenta auxiliar independente para testar o esquema.
Não importa arquivos proprietários, não gera sólidos e não executa scripts externos.

Exemplos:
 python python/component_tools.py evaluate samples/porta_parametrica.component.json
 python python/component_tools.py set samples/porta_parametrica.component.json nova_porta.json largura_m 1.20
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path

OPS = {"input", "constant", "add", "subtract", "multiply", "divide", "min", "max", "clamp"}


def evaluate_component(data: dict) -> dict[str, float]:
    definition = data["definition"]
    instance = data["instance"]
    if instance["definition_id"] != definition["id"]:
        raise ValueError("Instância não corresponde à definição")
    params = definition["parameters"]
    overrides = instance.get("overrides", {})
    if not set(overrides).issubset(params):
        raise ValueError("Instância contém parâmetro desconhecido")
    inputs: dict[str, float] = {}
    for key, item in params.items():
        value = overrides.get(key, item["value"])
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError(f"Valor inválido para {key}")
        if item.get("min") is not None and value < item["min"]:
            raise ValueError(f"{key}: valor menor que o mínimo")
        if item.get("max") is not None and value > item["max"]:
            raise ValueError(f"{key}: valor maior que o máximo")
        inputs[key] = float(value)
    graph = definition.get("graph")
    if graph is None:
        raise ValueError("Componente sem grafo paramétrico")
    if len(graph["nodes"]) > 4096:
        raise ValueError("Grafo grande demais")
    nodes = {node["id"]: node for node in graph["nodes"]}
    if len(nodes) != len(graph["nodes"]) or any(not name for name in nodes):
        raise ValueError("Nós repetidos ou com ID vazio")
    resolved: dict[str, float] = {}
    active: set[str] = set()

    def node_value(node_id: str, depth: int = 0) -> float:
        if node_id in resolved:
            return resolved[node_id]
        if depth > 256:
            raise ValueError("Grafo profundo demais")
        if node_id in active:
            raise ValueError(f"Dependência circular no nó {node_id}")
        if node_id not in nodes:
            raise ValueError(f"Nó desconhecido: {node_id}")
        active.add(node_id)
        node = nodes[node_id]
        op = node["op"]
        if op not in OPS:
            raise ValueError(f"Operação não suportada: {op}")
        if op == "input":
            value = inputs[node["parameter"]]
        elif op == "constant":
            value = node["value"]
        elif op == "clamp":
            number = node_value(node["value"], depth + 1)
            lower = node_value(node["min"], depth + 1)
            upper = node_value(node["max"], depth + 1)
            if lower > upper:
                raise ValueError("Clamp inválido")
            value = max(lower, min(number, upper))
        else:
            a = node_value(node["a"], depth + 1)
            b = node_value(node["b"], depth + 1)
            if op == "add": value = a + b
            elif op == "subtract": value = a - b
            elif op == "multiply": value = a * b
            elif op == "divide":
                if abs(b) < 1e-12: raise ValueError("Divisão por zero")
                value = a / b
            elif op == "min": value = min(a, b)
            else: value = max(a, b)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
            raise ValueError(f"Resultado não finito: {node_id}")
        resolved[node_id] = float(value)
        active.remove(node_id)
        return resolved[node_id]

    return {name: node_value(node_id) for name, node_id in graph["outputs"].items()}


def main() -> None:
    parser = argparse.ArgumentParser(description="Protótipo de grafo BIM — Projeto 2")
    subs = parser.add_subparsers(dest="cmd", required=True)
    evaluate = subs.add_parser("evaluate", help="Avaliar parâmetros e saídas do grafo")
    evaluate.add_argument("file", type=Path)
    change = subs.add_parser("set", help="Criar cópia com valor de instância atualizado")
    change.add_argument("file", type=Path)
    change.add_argument("output", type=Path)
    change.add_argument("parameter")
    change.add_argument("value", type=float)
    args = parser.parse_args()
    data = json.loads(args.file.read_text(encoding="utf-8"))
    if args.cmd == "set":
        if args.parameter not in data["definition"]["parameters"]:
            parser.error("Parâmetro não encontrado")
        data["instance"].setdefault("overrides", {})[args.parameter] = args.value
        outputs = evaluate_component(data)
        args.output.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"Cópia salva: {args.output}")
    else:
        outputs = evaluate_component(data)
    print(json.dumps(outputs, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
