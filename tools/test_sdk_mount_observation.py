"""Synthetic mount observations only; no TV, subprocess or filesystem mutation."""
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    "sdk_mount_observation", Path(__file__).with_name("sdk_mount_observation.py")
)
mounts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(mounts)


def row(target, options="ro,nosuid,relatime", source="/dev/example"):
    return f"{source} {target} ext4 {options} 0 0\n".encode()


class MountObservationTests(unittest.TestCase):
    def setUp(self):
        self.pair = b"".join(row(path) for path in mounts.PAIR_PATHS)
        self.baseline = b"".join(row(root + "/tmp", "rw,relatime", "tmpfs")
                                 for root in mounts.MOUNT_ROOTS) + self.pair
        self.shapes = {
            "/media/developer": ["rootfs / rootfs rw 0 0"],
            mounts.JAIL + "/media/developer": ["rootfs / rootfs rw 0 0"],
        }

    def grade(self, final=None, second=None, launches=2, shapes=None):
        final = self.baseline + self.pair * 2 if final is None else final
        return mounts.validate_sdk_mount_delta(
            self.baseline, final, final if second is None else second,
            controlled_own_launches=launches,
            cache_shapes_before=self.shapes,
            cache_shapes_after=self.shapes if shapes is None else shapes,
        )

    def test_preserves_all_four_views_and_reports_only_observed_exception(self):
        self.assertEqual(mounts.preview_scope_bytes(self.baseline), self.baseline)
        grade = self.grade()
        self.assertEqual(grade["mount_delta"]["appended_rows"], 4)
        self.assertFalse(grade["mounts_restored"])
        self.assertFalse(grade["mount_operations_performed"])
        self.assertFalse(grade["SDK_change_cause_qualified"])

    def test_exact_equality_needs_no_exception(self):
        self.assertTrue(self.grade(self.baseline, launches=0)["mounts_restored"])

    def test_foreign_source_and_sibling_target_never_select(self):
        unrelated = row(mounts.JAIL + "-other/tmp") + row("/unrelated", source=mounts.JAIL)
        self.assertEqual(mounts.preview_scope_bytes(unrelated + self.baseline), self.baseline)

    def test_unknown_same_app_alias_cannot_silently_leave_scope(self):
        with self.assertRaises(ValueError):
            self.grade(self.baseline + row("/unexpected" + mounts.JAIL + "/tmp"))

    def test_change_removal_or_reorder_of_any_original_view_refuses(self):
        lines = self.baseline.splitlines(keepends=True)
        for changed in (b"".join(lines[1:]), b"".join([lines[1], lines[0], *lines[2:]]),
                        self.baseline.replace(b"rw,relatime", b"ro,relatime", 1)):
            with self.subTest(changed=changed[:60]), self.assertRaises(ValueError):
                self.grade(changed)

    def test_only_existing_canonical_readonly_pair_can_append(self):
        a, b = self.pair.splitlines(keepends=True)
        wrongs = (a, b + a, self.pair.replace(b"ro,", b"rw,"),
                  self.pair.replace(b"/dev/example", b"/dev/different"),
                  self.pair.replace(mounts.JAIL.encode(),
                                    ("/mnt/lg/user" + mounts.JAIL).encode()),
                  row(mounts.JAIL + "/foreign"))
        for suffix in wrongs:
            with self.subTest(suffix=suffix[:60]), self.assertRaises(ValueError):
                self.grade(self.baseline + suffix)

    def test_launch_bound_is_finite_literal_data_not_boolean(self):
        for count in (True, -1, 3, 1):
            with self.subTest(count=count), self.assertRaises(ValueError):
                self.grade(launches=count)

    def test_unstable_full_snapshot_or_changed_cache_shape_refuses(self):
        with self.assertRaises(ValueError):
            self.grade(second=self.baseline)
        with self.assertRaises(ValueError):
            self.grade(shapes={})
        changed = dict(self.shapes)
        changed["/media/developer"] = ["rootfs / rootfs rw 0 0", "extra /media/developer ext4 rw 0 0"]
        with self.assertRaises(ValueError):
            self.grade(shapes=changed)

    def test_incomplete_escaped_or_malformed_observations_refuse(self):
        for final in (self.baseline[:-1], self.baseline.replace(b"\n", b"\r\n"),
                      self.baseline + b"bad row\n", self.baseline.replace(b"openssh", b"open\\040ssh")):
            with self.subTest(final=final[:60]), self.assertRaises(ValueError):
                self.grade(final)


if __name__ == "__main__":
    unittest.main()
