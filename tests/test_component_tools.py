"""Testes do protótipo de componentes/graph: independentes do Revit e dos SDKs."""
import json
import sys
import unittest
from copy import deepcopy
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from component_tools import evaluate_component


class ComponentGraphTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.sample = json.loads((ROOT / "samples" / "porta_parametrica.component.json").read_text(encoding="utf-8"))

    def test_original_outputs(self):
        result = evaluate_component(self.sample)
        self.assertAlmostEqual(result["largura_folha_m"], 0.86)
        self.assertAlmostEqual(result["area_folha_m2"], 1.806)

    def test_instance_override_recalculates(self):
        obj = deepcopy(self.sample)
        obj["instance"]["overrides"]["largura_m"] = 1.20
        self.assertAlmostEqual(evaluate_component(obj)["largura_folha_m"], 1.16)
        self.assertEqual(self.sample["definition"]["parameters"]["largura_m"]["value"], 0.9)

    def test_invalid_override_rejected(self):
        obj = deepcopy(self.sample)
        obj["instance"]["overrides"]["largura_m"] = 5.0
        with self.assertRaises(ValueError): evaluate_component(obj)

    def test_cycles_rejected(self):
        obj = deepcopy(self.sample)
        obj["definition"]["graph"]["nodes"][0] = {"id": "n_largura", "op": "add", "a": "n_folha", "b": "n_folga"}
        with self.assertRaisesRegex(ValueError, "circular"):
            evaluate_component(obj)

    def test_missing_nodes_rejected(self):
        obj = deepcopy(self.sample)
        obj["definition"]["graph"]["outputs"]["largura_folha_m"] = "inexistente"
        with self.assertRaisesRegex(ValueError, "desconhecido"):
            evaluate_component(obj)

    def test_repeated_nodes_rejected(self):
        obj = deepcopy(self.sample)
        obj["definition"]["graph"]["nodes"].append(obj["definition"]["graph"]["nodes"][0])
        with self.assertRaisesRegex(ValueError, "repetidos"):
            evaluate_component(obj)


if __name__ == "__main__": unittest.main()
