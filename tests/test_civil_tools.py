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
