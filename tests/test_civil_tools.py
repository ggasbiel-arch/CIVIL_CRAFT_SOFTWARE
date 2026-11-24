import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "python"))
from civil_tools import project_summary, read_project, validate_project  # noqa: E402


class TestProjectInspection(unittest.TestCase):
    def setUp(self):
        self.project = read_project(ROOT / "samples" / "projeto_vazio.bim.json")

    def test_blank_project_is_valid(self):
        validate_project(self.project)
        self.assertEqual(project_summary(self.project)["elements"], 0)

    def test_level_metadata(self):
        self.assertEqual(project_summary(self.project)["levels"], 1)

    def test_duplicate_level_id_is_rejected(self):
        self.project["levels"].append(json.loads(json.dumps(self.project["levels"][0])))
        with self.assertRaises(ValueError):
            validate_project(self.project)

    def test_legacy_schema_remains_readable(self):
        self.project["elements"].append({
            "type": "wall", "id": "W1", "level_id": "P0",
            "start": [0.0, 0.0], "end": [2.0, 0.0],
            "thickness_m": 0.2, "height_m": 2.8
        })
        validate_project(self.project)
        self.assertEqual(project_summary(self.project)["elements"], 1)


if __name__ == "__main__":
    unittest.main()
