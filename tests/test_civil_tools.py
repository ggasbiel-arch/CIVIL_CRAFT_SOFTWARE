import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from civil_tools import add_wall, quantities, read_project, validate_project  # noqa: E402


class TestAutomation(unittest.TestCase):
    def setUp(self):
        self.project = read_project(ROOT / "samples" / "casa_piloto.bim.json")

    def test_example_quantities(self):
        self.assertAlmostEqual(quantities(self.project)["wall_length_m"], 28)
        self.assertAlmostEqual(quantities(self.project)["wall_gross_volume_m3"], 15.68)

    def test_add_wall(self):
        wall_id = add_wall(self.project, "P0", [0, 0], [2, 0], 0.15, 2.8)
        self.assertEqual(wall_id, "W5")
        validate_project(self.project)

    def test_add_wall_preserves_components_and_avoids_id_collision(self):
        with (ROOT / "samples" / "porta_parametrica.component.json").open(encoding="utf-8") as stream:
            door = json.load(stream)
        definition = door["definition"]
        instance = {**door["instance"], "id": "W5", "level_id": "P0"}
        self.project["component_definitions"] = {definition["id"]: definition}
        self.project["component_instances"] = [instance]

        wall_id = add_wall(self.project, "P0", [0, 0], [2, 0], 0.15, 2.8)

        self.assertEqual(wall_id, "W6")
        self.assertEqual(self.project["component_instances"], [instance])
        self.assertEqual(self.project["component_definitions"], {definition["id"]: definition})

    def test_invalid_wall_is_rejected_without_side_effects(self):
        original = len(self.project["elements"])
        with self.assertRaises(ValueError):
            add_wall(self.project, "P0", [0, 0], [0, 0], 0.2, 2.8)
        self.assertEqual(len(self.project["elements"]), original)

    def test_duplicate_id_is_rejected(self):
        self.project["elements"].append(json.loads(json.dumps(self.project["elements"][0])))
        with self.assertRaises(ValueError):
            validate_project(self.project)


if __name__ == "__main__":
    unittest.main()
