import copy
import json
import unittest

import import_github as importer


class ImporterTests(unittest.TestCase):
    def setUp(self):
        self.data = json.loads(importer.MANIFEST.read_text(encoding="utf-8"))

    def test_authoritative_manifest_is_valid(self):
        importer.validate(self.data)
        self.assertEqual(len(self.data["labels"]), 25)
        self.assertEqual(len(self.data["milestones"]), 8)
        self.assertEqual(len(self.data["issues"]), 65)

    def test_invalid_category_and_cycles_are_rejected(self):
        invalid = copy.deepcopy(self.data)
        invalid["issues"][0]["labels"][0] = "priority:P1"
        with self.assertRaises(ValueError):
            importer.validate(invalid)
        self.data["issues"][0]["depends_on"] = ["F02"]
        with self.assertRaisesRegex(ValueError, "cycle"):
            importer.validate(self.data)

    def test_linking_preserves_contributor_prose_and_checked_criteria(self):
        spec = self.data["issues"][1]
        current = importer.body(spec).replace("- [ ]", "- [x]", 1)
        current += "\nContributor investigation.\n"
        mapping = {"F01": {"html_url": "https://github.com/pyfaro/pyfaro/issues/1"}}
        linked = importer.linked_body(spec, {"body": current}, mapping)
        self.assertIn("- [x]", linked)
        self.assertIn("Contributor investigation.", linked)
        self.assertIn("- `F01`: https://github.com/pyfaro/pyfaro/issues/1", linked)
        self.assertEqual(
            importer.linked_body(spec, {"body": linked}, mapping), linked
        )

    def test_resume_uses_id_even_when_title_was_edited(self):
        spec = self.data["issues"][0]
        remote = {"title": "Edited", "body": importer.body(spec),
                  "id": 123, "number": 1}
        self.assertEqual(importer.identify([spec], [remote]), {"F01": remote})
        with self.assertRaisesRegex(ValueError, "Duplicate remote backlog"):
            importer.identify([spec], [remote, {**remote, "number": 2}])

    def test_unmanaged_title_collision_is_not_adopted(self):
        spec = self.data["issues"][0]
        with self.assertRaisesRegex(ValueError, "Unmanaged title collision"):
            importer.identify([spec], [{"title": spec["title"], "body": "Hi",
                                       "id": 123, "number": 1}])

    def test_edited_dependency_section_is_not_overwritten(self):
        spec = self.data["issues"][1]
        remote = {"body": importer.body(spec).replace("`F01`", "custom")}
        mapping = {"F01": {"html_url": "https://github.com/pyfaro/pyfaro/issues/1"}}
        with self.assertRaisesRegex(ValueError, "Edited dependency section"):
            importer.linked_body(spec, remote, mapping)


if __name__ == "__main__":
    unittest.main()
