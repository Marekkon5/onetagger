import unittest
from cache_cleanup import cache_pr, eligible


class CacheScopeTests(unittest.TestCase):
    def cache(self, ref, key="v1-onetagger-deps-macos-arm64-abc"):
        return {"ref": ref, "key": key}

    def test_protect_default_branch_open_prs_and_unrelated_caches(self):
        for cache in [self.cache("refs/heads/master"), self.cache("refs/pull/22/merge"),
                      self.cache("refs/pull/34/head"), self.cache("refs/pull/34/merge", "pnpm-store")]:
            with self.subTest(cache=cache):
                self.assertFalse(eligible(cache, {34}))

    def test_closed_pr_selection_and_manual_sweep(self):
        cache = self.cache("refs/pull/34/merge")
        self.assertTrue(eligible(cache, {34}))
        self.assertTrue(eligible(cache, {34}, 34))
        self.assertFalse(eligible(cache, {34}, 27))
        self.assertFalse(eligible(cache, set()))

    def test_reject_malformed_refs_and_missing_fields(self):
        for ref in ["refs/pull/0/merge", "refs/pull/-1/merge", "refs/pull/34/merge/extra", "refs/pull/34/merge\n"]:
            self.assertIsNone(cache_pr(self.cache(ref)))
        self.assertIsNone(cache_pr({}))


if __name__ == "__main__":
    unittest.main()
