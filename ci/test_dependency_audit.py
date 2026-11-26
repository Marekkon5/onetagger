import unittest
from dependency_audit import findings


class AdvisoryComparisonTests(unittest.TestCase):
    def test_the_same_advisory_on_an_updated_package_is_still_visible(self):
        before = findings('cargo', {'vulnerabilities': {'list': [
            {'package': {'name': 'fixture', 'version': '1'}, 'advisory': {'id': 'RUSTSEC-fixture'}}]}})
        after = findings('cargo', {'vulnerabilities': {'list': [
            {'package': {'name': 'fixture', 'version': '2'}, 'advisory': {'id': 'RUSTSEC-fixture'}}]}})
        self.assertEqual(before, after)
        self.assertEqual(len(after), 1)

    def test_registry_errors_are_not_empty_advisory_sets(self):
        with self.assertRaises(KeyError):
            findings('pnpm', {'error': 'registry unavailable'})

    def test_shared_advisory_on_a_new_package_is_an_introduced_finding(self):
        data = {'advisories': {'1': {'module_name': 'old', 'id': 1, 'github_advisory_id': 'GHSA-fixture'},
                               '2': {'module_name': 'new', 'id': 2, 'github_advisory_id': 'GHSA-fixture'}}}
        self.assertEqual(findings('pnpm', data), {('old', 'GHSA-fixture'), ('new', 'GHSA-fixture')})
