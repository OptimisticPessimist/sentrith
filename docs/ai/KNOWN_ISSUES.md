# Known Engineering Issues

> Recurring or expensive-to-rediscover failures, traps, and confirmed workarounds.
> Do not duplicate the normal issue tracker.

## Entry template

### ISSUE-YYYYMMDD-NN — Short symptom

**Status:** active | mitigated | resolved

**Symptoms**

- Exact error, unexpected behavior, or observable failure.

**Affected area**

- Component:
- Versions/platforms:
- Relevant files:

**Root cause**

Confirmed cause. If not confirmed, write `Unknown`.

**Diagnosis**

How to verify that this is the same problem.

```sh
# relevant diagnostic command if useful
```

**Fix / workaround**

Steps confirmed to work.

**Do not do**

Tempting fixes that were tested and found ineffective or harmful.

**References**

- source path
- regression test
- upstream issue/documentation

---

### macOS temporary paths and no-follow directory checks

Storage tests can fail with `/var is not a real directory` because macOS temporary paths traverse `/var -> /private/var`. Canonicalize the test-owned root after creating it (see `storage::tests::temp_root`); keep production data-path symlink rejection intact. The existing directory-boundary tests use repository-relative paths for the same reason.
