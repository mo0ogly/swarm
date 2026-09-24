import copy
import json
import unittest
from fragment_preflight import plan, validate


def patch(name, lines=2000):
    return (f'diff --git a/{name} b/{name}\nnew file mode 100644\n--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,{lines} @@\n' + '+évidence partagée\n'*lines).encode()


class FragmentPreflightTest(unittest.TestCase):
    def test_lossless_bounded_deterministic(self):
        diff = b''.join(patch(str(i)) for i in range(15))
        result, packets = plan(diff, {'candidate':'a','base':'b'}, 12, 2)
        self.assertGreater(len(packets), 1)
        self.assertEqual((result, packets), plan(diff, {'candidate':'a','base':'b'}, 12, 2))
        validate(diff, result, packets)
        self.assertEqual(result['files'], 15)

    def test_budget_and_indivisible_file(self):
        with self.assertRaises(ValueError):
            plan(b''.join(patch(str(i)) for i in range(15)), {}, 3, 2)
        with self.assertRaises(ValueError):
            plan(patch('huge', 15000), {}, 12, 2)

    def test_binary_metadata_not_source_literal(self):
        plan(patch('a',1)+b'+Binary files sample\n', {}, 12, 2)
        for value in [b'Binary files a/x and b/x differ\n', b'GIT binary patch\n']:
            with self.assertRaises(ValueError):
                plan(b'diff --git a/x b/x\n'+value, {}, 12, 2)

    def test_tampered_missing_and_reordered_packets(self):
        diff=b''.join(patch(str(i)) for i in range(15))
        result, packets=plan(diff, {'candidate':'a'}, 12, 2)
        for bad in [packets[:-1], list(reversed(packets)), [packets[0]+b' ']+packets[1:]]:
            with self.assertRaises(ValueError): validate(diff,result,bad)
        changed=copy.deepcopy(result);changed['identity']['candidate']='other'
        with self.assertRaises(ValueError): validate(diff,changed,packets)

    def test_duplicate_and_invalid_text(self):
        for diff in [patch('x')*2,b'not a patch',b'diff --git a/x b/x\n+\xff\n']:
            with self.assertRaises((ValueError,UnicodeError)):plan(diff,{},12,2)


if __name__ == '__main__': unittest.main()
