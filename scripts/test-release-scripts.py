#!/usr/bin/env python3
"""Tests for the release and build-provenance scripts: what a contract's
release scope covers (scripts/release-deps.py, used by release.yml's
release-pr job), that release.yml publishes every npm package under
packages/ with a manifest that installs from the registry, an npm package's
scope and version bump (scripts/npm-release.sh, used by release-pr and by
deploy-stack's perch-artifacts bump), the Nido dispatch plan
(scripts/nido-dispatch-plan.sh), that no release job holding a signing
credential builds or puts it in argv, what a stack build records as dirty
(scripts/source-dirty.sh, used by scripts/build-stack.sh), and the committed
build-time pins' drift check (scripts/check-pins.sh, with
scripts/manifest-registry.sh).

  python3 scripts/test-release-scripts.py

The git-cliff tests need `git-cliff` on PATH (CI installs the release job's
pinned build); without it they are skipped unless PERCH_REQUIRE_GIT_CLIFF=1.
"""

import base64
import hashlib
import http.server
import io
import json
import os
import re
import shlex
import shutil
import subprocess
import tarfile
import tempfile
import textwrap
import threading
import unittest

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RELEASE_DEPS = os.path.join(REPO, "scripts", "release-deps.py")
SOURCE_DIRTY = os.path.join(REPO, "scripts", "source-dirty.sh")
CHECK_PINS = os.path.join(REPO, "scripts", "check-pins.sh")
MANIFEST_REGISTRY = os.path.join(REPO, "scripts", "manifest-registry.sh")
NPM_RELEASE = os.path.join(REPO, "scripts", "npm-release.sh")
NIDO_PLAN = os.path.join(REPO, "scripts", "nido-dispatch-plan.sh")
RELEASE_YML = os.path.join(REPO, ".github", "workflows", "release.yml")
GIT_CLIFF = shutil.which("git-cliff")


def run(cmd, cwd, check=True):
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, check=check)


class Repo:
    def __init__(self):
        self.dir = tempfile.mkdtemp()
        run(["git", "init", "-q"], self.dir)
        run(["git", "config", "user.email", "t@example.com"], self.dir)
        run(["git", "config", "user.name", "t"], self.dir)
        run(["git", "config", "commit.gpgsign", "false"], self.dir)

    def write(self, path, text):
        full = os.path.join(self.dir, path)
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "w") as f:
            f.write(textwrap.dedent(text))

    def commit(self, message):
        run(["git", "add", "-A"], self.dir)
        run(["git", "commit", "-q", "--allow-empty", "-m", message], self.dir)

    def deps(self, *args):
        return run(["python3", RELEASE_DEPS, *args], self.dir).stdout.splitlines()

    def cleanup(self):
        shutil.rmtree(self.dir)


# A workspace whose contract links lib (which links a vendored path crate)
# and two external pins; a tool crate's and a dev dependency's packages are
# in the lockfile but not linked by the contract.
DEFAULTS = dict(
    oz_rev="aaaaaaaaaaaaaaaa",
    sdk_features="",
    macros="1.0.0",
    field="3.0.0",
    devonly="1.0.0",
    toolonly="1.0.0",
    member_version="0.1.0",
    members='"crates/contract", "crates/lib", "crates/tool"',
    opt_level="z",
    sdk_deps='"sdk-macros"',
)


def workspace(repo, **over):
    v = {**DEFAULTS, **over}
    sdk = f'{{ version = "1.0.0", features = [{v["sdk_features"]}] }}' if v["sdk_features"] else '"1.0.0"'
    repo.write("Cargo.toml", f"""\
        [workspace]
        members = [{v["members"]}]

        [workspace.dependencies]
        sdk = {sdk}
        oz = {{ version = "0.7.1", git = "https://example.com/oz.git", rev = "{v["oz_rev"]}", package = "oz-accounts" }}
        lib = {{ version = "{v["member_version"]}", path = "crates/lib" }}

        [profile.release]
        opt-level = "{v["opt_level"]}"
        """)
    repo.write("crates/contract/Cargo.toml", f"""\
        [package]
        name = "contract"
        version = "{v["member_version"]}"

        [dependencies]
        lib = {{ workspace = true }}
        sdk = {{ workspace = true }}
        oz = {{ workspace = true }}

        [dev-dependencies]
        devonly = "1.0.0"
        """)
    repo.write("crates/lib/Cargo.toml", f"""\
        [package]
        name = "lib"
        version = "{v["member_version"]}"

        [target.'cfg(target_family = "wasm")'.dependencies]
        v = {{ path = "../../vendor/v" }}
        """)
    repo.write("vendor/v/Cargo.toml", """\
        [package]
        name = "v"
        version = "0.1.0"

        [dependencies]
        field = "3"
        """)
    repo.write("crates/tool/Cargo.toml", f"""\
        [package]
        name = "tool"
        version = "{v["member_version"]}"

        [dependencies]
        sdk = {{ workspace = true }}
        toolonly = "1"
        """)
    reg = 'source = "registry+https://github.com/rust-lang/crates.io-index"'
    repo.write("Cargo.lock", f"""\
        version = 3

        [[package]]
        name = "contract"
        version = "{v["member_version"]}"
        dependencies = ["devonly", "lib", "oz-accounts", "sdk"]

        [[package]]
        name = "lib"
        version = "{v["member_version"]}"
        dependencies = ["v"]

        [[package]]
        name = "v"
        version = "0.1.0"
        dependencies = ["field"]

        [[package]]
        name = "tool"
        version = "{v["member_version"]}"
        dependencies = ["sdk", "toolonly"]

        [[package]]
        name = "sdk"
        version = "1.0.0"
        {reg}
        dependencies = [{v["sdk_deps"]}]

        [[package]]
        name = "pem"
        version = "0.7.0"
        {reg}

        [[package]]
        name = "sdk-macros"
        version = "{v["macros"]}"
        {reg}

        [[package]]
        name = "oz-accounts"
        version = "0.7.1"
        source = "git+https://example.com/oz.git?rev={v["oz_rev"]}#{v["oz_rev"]}"
        dependencies = ["sdk"]

        [[package]]
        name = "field"
        version = "{v["field"]}"
        {reg}

        [[package]]
        name = "devonly"
        version = "{v["devonly"]}"
        {reg}

        [[package]]
        name = "toolonly"
        version = "{v["toolonly"]}"
        {reg}
        """)


class ReleaseDeps(unittest.TestCase):
    def setUp(self):
        self.repo = Repo()
        workspace(self.repo)
        self.repo.write("rust-toolchain.toml", '[toolchain]\nchannel = "stable"\n')
        self.repo.commit("feat(contract): first release")

    def tearDown(self):
        self.repo.cleanup()

    def changed(self, extra=None, **over):
        workspace(self.repo, **over)
        if extra:
            extra(self.repo)
        self.repo.commit("chore(deps): change")
        return self.repo.deps("changes", "contract", "HEAD~1", "HEAD")

    def test_a_dependency_re_pin_is_a_change(self):
        # The OZ fork re-pin of #111: Cargo.toml and Cargo.lock only.
        self.assertEqual(
            self.changed(oz_rev="bbbbbbbbbbbbbbbb"),
            ["oz-accounts 0.7.1 (git aaaaaaaaaaaa) -> 0.7.1 (git bbbbbbbbbbbb)"],
        )

    def test_a_transitive_dependency_is_a_change(self):
        self.assertEqual(self.changed(macros="1.0.1"), ["sdk-macros 1.0.0 -> 1.0.1"])

    def test_a_vendored_crates_dependency_is_a_change(self):
        self.assertEqual(self.changed(field="3.1.0"), ["field 3.0.0 -> 3.1.0"])

    def test_inherited_features_are_a_change(self):
        self.assertEqual(self.changed(sdk_features='"testutils"'), ["workspace dependency sdk changed"])

    def test_the_release_profile_and_the_toolchain_are_changes(self):
        self.assertEqual(self.changed(opt_level="s"), ["Cargo.toml [profile] changed"])

        def toolchain(r):
            r.write("rust-toolchain.toml", '[toolchain]\nchannel = "1.97.1"\n')
            r.write(".cargo/config.toml", '[build]\nrustflags = ["-Cdebuginfo=0"]\n')

        self.assertEqual(self.changed(extra=toolchain, opt_level="s"), ["rust-toolchain.toml changed", ".cargo/config.toml changed"])

    def test_what_the_contract_does_not_link_is_no_change(self):
        # A release commit's member versions, another crate's dependency, a
        # dev dependency, and a new workspace member.
        self.assertEqual(self.changed(member_version="0.2.0"), [])
        self.assertEqual(self.changed(member_version="0.2.0", toolonly="1.5.0"), [])
        self.assertEqual(self.changed(member_version="0.2.0", devonly="2.0.0"), [])

        def other(r):
            r.write("crates/other/Cargo.toml", '[package]\nname = "other"\nversion = "0.1.0"\n')

        members = DEFAULTS["members"] + ', "crates/other"'
        self.assertEqual(self.changed(extra=other, member_version="0.2.0", members=members), [])
        # A package entering the walk only through another crate's feature
        # (the lockfile is the union of every crate's features).
        self.assertEqual(self.changed(member_version="0.2.0", members=members, extra=other, sdk_deps='"pem", "sdk-macros"'), [])

    def test_the_scope_is_every_path_package_linked(self):
        self.assertEqual(self.repo.deps("scope", "contract"), ["crates/contract", "crates/lib", "vendor/v"])

    def test_cliff_args_count_from_the_latest_tag(self):
        run(["git", "tag", "contract-v0.1.0"], self.repo.dir)
        # Another crate whose name extends this one is not its tag.
        run(["git", "tag", "contract-factory-v9.0.0"], self.repo.dir)
        self.assertEqual(self.repo.deps("cliff-args", "contract"), [])
        workspace(self.repo, oz_rev="bbbbbbbbbbbbbbbb")
        self.repo.commit("chore(deps): re-pin oz")
        run(["git", "tag", "contract-v0.1.1"], self.repo.dir)
        workspace(self.repo, oz_rev="cccccccccccccccc")
        self.repo.commit("chore(deps): re-pin oz again")
        self.assertEqual(
            self.repo.deps("cliff-args", "contract"),
            ["--with-commit", "fix(deps): oz-accounts 0.7.1 (git bbbbbbbbbbbb) -> 0.7.1 (git cccccccccccc)"],
        )

    @unittest.skipUnless(GIT_CLIFF or os.environ.get("PERCH_REQUIRE_GIT_CLIFF"), "git-cliff not on PATH")
    def test_git_cliff_bumps_a_contract_whose_dependencies_moved(self):
        # release.yml's invocation, on a `chore(deps)` re-pin that touches no
        # crate tree: git-cliff alone finds nothing to bump. (The tag is on a
        # commit in the scope, as a release commit is.)
        shutil.copy(os.path.join(REPO, "cliff.toml"), os.path.join(self.repo.dir, ".git", "cliff.toml"))
        run(["git", "tag", "contract-v0.1.0"], self.repo.dir)
        workspace(self.repo, oz_rev="bbbbbbbbbbbbbbbb")
        self.repo.commit("chore(deps): re-pin oz")
        scope = [a for d in self.repo.deps("scope", "contract") for a in ("--include-path", f"{d}/**")]
        cliff = [GIT_CLIFF or "git-cliff", "--config", ".git/cliff.toml", "--tag-pattern", "contract-v.*", *scope]
        self.assertEqual(run([*cliff, "--bumped-version"], self.repo.dir).stdout.strip(), "contract-v0.1.0")
        deps = self.repo.deps("cliff-args", "contract")
        self.assertEqual(run([*cliff, *deps, "--bumped-version"], self.repo.dir).stdout.strip(), "contract-v0.1.1")
        changelog = run([*cliff, *deps, "--bump"], self.repo.dir).stdout
        self.assertIn("## [0.1.1]", changelog)
        self.assertIn("Oz-accounts 0.7.1 (git aaaaaaaaaaaa) -> 0.7.1 (git bbbbbbbbbbbb)", changelog)

    @unittest.skipUnless(GIT_CLIFF or os.environ.get("PERCH_REQUIRE_GIT_CLIFF"), "git-cliff not on PATH")
    def test_a_pin_refresh_is_no_release(self):
        # The deploy(<network>) commit rewrites the contract's committed
        # build-time pins after a release built new bytes: release.yml's
        # `pins` exclusion keeps it from releasing the contract again.
        shutil.copy(os.path.join(REPO, "cliff.toml"), os.path.join(self.repo.dir, ".git", "cliff.toml"))
        run(["git", "tag", "contract-v0.1.0"], self.repo.dir)
        self.repo.write("crates/contract/wasm/dep.sha256", "bb\n")
        self.repo.write("deployments/testnet.json", "{}\n")
        self.repo.commit("deploy(testnet): recovery stack")
        scope = [a for d in self.repo.deps("scope", "contract") for a in ("--include-path", f"{d}/**")]
        pins = shlex.split(re.search(r"^ *pins=\((.*)\)$", release_yml(), re.M).group(1))
        cliff = [GIT_CLIFF or "git-cliff", "--config", ".git/cliff.toml", "--tag-pattern", "contract-v.*", *scope]
        self.assertEqual(run([*cliff, "--bumped-version"], self.repo.dir).stdout.strip(), "contract-v0.1.1")
        self.assertEqual(run([*cliff, *pins, "--bumped-version"], self.repo.dir).stdout.strip(), "contract-v0.1.0")

    @unittest.skipUnless(GIT_CLIFF or os.environ.get("PERCH_REQUIRE_GIT_CLIFF"), "git-cliff not on PATH")
    def test_a_registry_path_edit_is_a_release(self):
        # The registry path sits beside the hash pins but is compiled in and
        # rewritten by no deployment: editing it changes the contract's
        # bytes, so release.yml's `pins` exclusion leaves it in.
        shutil.copy(os.path.join(REPO, "cliff.toml"), os.path.join(self.repo.dir, ".git", "cliff.toml"))
        run(["git", "tag", "contract-v0.1.0"], self.repo.dir)
        self.repo.write("crates/contract/wasm/registry.path", "root-deployer G\nroot-salt s\npath a/b\n")
        self.repo.commit("fix(contract): move the registry")
        scope = [a for d in self.repo.deps("scope", "contract") for a in ("--include-path", f"{d}/**")]
        pins = shlex.split(re.search(r"^ *pins=\((.*)\)$", release_yml(), re.M).group(1))
        cliff = [GIT_CLIFF or "git-cliff", "--config", ".git/cliff.toml", "--tag-pattern", "contract-v.*", *scope]
        self.assertEqual(run([*cliff, *pins, "--bumped-version"], self.repo.dir).stdout.strip(), "contract-v0.1.1")


def release_yml():
    with open(RELEASE_YML) as f:
        return f.read()


def paths_for(contract):
    """release.yml's `paths_for()` for `contract`, as repository paths."""
    yml = release_yml()
    fn = re.search(r"^ *(paths_for\(\) \{ case .*?esac; \})", yml, re.S | re.M).group(1)
    out = run(["bash", "-c", f'{fn}\npaths_for "$1"', "_", contract], REPO).stdout.split()
    return {d if "/" in d else f"crates/{d}" for d in out}


def contract_lists():
    return re.findall(r'^ *CONTRACTS="([^"]*)"', release_yml(), re.M)


class ReleaseScope(unittest.TestCase):
    """The workflow's scopes against this repository's own workspace."""

    def test_both_contract_lists_agree(self):
        lists = contract_lists()
        self.assertEqual(len(lists), 2)
        self.assertEqual(lists[0].split(), lists[1].split())

    def test_every_crate_a_contract_links_is_in_its_scope(self):
        for contract in contract_lists()[0].split():
            linked = set(run(["python3", RELEASE_DEPS, "scope", contract], REPO).stdout.split())
            missing = linked - paths_for(contract)
            self.assertFalse(missing, f"{contract} links {sorted(missing)}, outside its paths_for() scope")

    def test_every_contract_links_the_sdk(self):
        # The lockfile walk reaches the real dependency graph.
        for contract in contract_lists()[0].split():
            tree = run(["python3", "-c", f"""
import importlib.util, sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("rd", {RELEASE_DEPS!r})
rd = importlib.util.module_from_spec(spec); spec.loader.exec_module(rd)
_, ext, _ = rd.Tree("HEAD").closure({contract!r})
print(" ".join(sorted({{n for n, _, _ in ext}})))
"""], REPO).stdout.split()
            self.assertIn("soroban-sdk", tree, contract)


def npm_lists():
    return re.findall(r'^ *NPM_PACKAGES="([^"]*)"', release_yml(), re.M)


def npm_paths_for(pkg, cwd=REPO):
    """scripts/npm-release.sh's scope for `pkg`, as the git-cliff
    --include-path arguments its `bump` builds from it in `cwd`."""
    paths = run([NPM_RELEASE, "paths", pkg], cwd).stdout.split()
    return paths, [a for d in paths for a in ("--include-path", f"{d}/**" if os.path.isdir(os.path.join(cwd, d)) else d)]


class NpmScope(unittest.TestCase):
    """The workflow's npm package lists against packages/."""

    def test_both_lists_name_every_package(self):
        lists = npm_lists()
        self.assertEqual(len(lists), 2)
        on_disk = sorted(os.listdir(os.path.join(REPO, "packages")))
        for listed in lists:
            self.assertEqual(sorted(listed.split()), on_disk)

    def test_every_package_installs_from_the_registry(self):
        for pkg in npm_lists()[0].split():
            with open(os.path.join(REPO, "packages", pkg, "package.json")) as f:
                manifest = json.load(f)
            self.assertTrue(manifest["name"].startswith("@stellar-registry/"), pkg)
            self.assertEqual(manifest.get("publishConfig", {}).get("access"), "public", pkg)
            self.assertEqual(manifest["repository"]["url"], "https://github.com/stellar-registry/perch", pkg)
            self.assertEqual(manifest["repository"]["directory"], f"packages/{pkg}", pkg)
            for field in ("dependencies", "peerDependencies", "optionalDependencies"):
                for dep, spec in manifest.get(field, {}).items():
                    self.assertFalse(
                        re.match(r"(file|link|workspace|git\+?\w*):|\.{0,2}/", spec),
                        f"{pkg} {field} {dep}@{spec} does not resolve from the registry",
                    )


    def test_every_package_is_scoped_to_what_it_ships(self):
        # A package that copies repository files at build time
        # (packages/<pkg>/sources.json) re-releases when one of them changes.
        for pkg in npm_lists()[0].split():
            want = {f"packages/{pkg}"}
            sources = os.path.join(REPO, "packages", pkg, "sources.json")
            if os.path.exists(sources):
                with open(sources) as f:
                    want |= set(json.load(f).values())
            paths, _ = npm_paths_for(pkg)
            self.assertEqual(sorted(paths), sorted(want), pkg)
            for d in paths:
                self.assertTrue(os.path.exists(os.path.join(REPO, d)), f"{pkg} scope names missing {d}")

    @unittest.skipUnless(GIT_CLIFF or os.environ.get("PERCH_REQUIRE_GIT_CLIFF"), "git-cliff not on PATH")
    def test_git_cliff_bumps_a_package_when_a_file_it_ships_changes(self):
        repo = Repo()
        self.addCleanup(repo.cleanup)
        shutil.copy(os.path.join(REPO, "cliff.toml"), os.path.join(repo.dir, ".git", "cliff.toml"))
        repo.write("packages/perch-artifacts/package.json", "{}")
        repo.write("deployments/testnet.json", "{}")
        repo.write("testdata/rule-hashes.json", "{}")
        repo.write("testdata/other.json", "{}")
        repo.commit("feat: initial")
        run(["git", "tag", "perch-artifacts-v0.1.0"], repo.dir)
        _, inc = npm_paths_for("perch-artifacts", repo.dir)
        cliff = [GIT_CLIFF or "git-cliff", "--config", ".git/cliff.toml", "--tag-pattern", "perch-artifacts-v.*", *inc, "--bumped-version"]
        repo.write("testdata/other.json", '{"a": 1}')
        repo.commit("fix: a vector the package does not ship")
        self.assertEqual(run(cliff, repo.dir).stdout.strip(), "perch-artifacts-v0.1.0")
        repo.write("testdata/rule-hashes.json", '{"a": 1}')
        repo.commit("fix: a shipped vector")
        self.assertEqual(run(cliff, repo.dir).stdout.strip(), "perch-artifacts-v0.1.1")
        repo.write("deployments/testnet.json", '{"a": 1}')
        repo.commit("feat: redeploy")
        self.assertEqual(run(cliff, repo.dir).stdout.strip(), "perch-artifacts-v0.2.0")


def release_jobs():
    """release.yml's jobs, as {name: the job's YAML text}."""
    body = release_yml().split("\njobs:\n", 1)[1]
    parts = re.split(r"^  ([A-Za-z0-9_-]+):\n", body, flags=re.M)
    return dict(zip(parts[1::2], parts[2::2]))


SEED = "secrets.CI_PUBLISH_SECRET_KEY"
# What compiles or runs a dependency's code: a job holding a signing
# credential must do none of it.
BUILDS = ("cargo ", "rust-toolchain", "npm ci", "npm install", "npm test", "npm run",
          "--build-only", 'scaffold: "true"', "verify-deployment.sh", "bindings-contracts.sh")


class ReleaseKeys(unittest.TestCase):
    """Where release.yml's signing credentials can be read."""

    def test_no_job_reads_an_npm_token(self):
        self.assertNotIn("NPM_TOKEN", release_yml())
        self.assertNotIn("NODE_AUTH_TOKEN", release_yml())

    def test_every_job_holding_the_seed_is_in_an_environment(self):
        holders = {n: j for n, j in release_jobs().items() if SEED in j}
        self.assertTrue(holders)
        for name, job in holders.items():
            with self.subTest(job=name):
                # A job's own `environment:`, or the reusable publish's input.
                self.assertRegex(job, r"(?m)^ {4,6}environment: \S")

    def test_no_job_holding_the_seed_builds(self):
        for name, job in release_jobs().items():
            if SEED not in job:
                continue
            for marker in BUILDS:
                with self.subTest(job=name, marker=marker):
                    self.assertNotIn(marker, job)

    def test_the_seed_never_reaches_argv(self):
        for name, job in release_jobs().items():
            for line in job.splitlines():
                if SEED in line:
                    with self.subTest(job=name, line=line.strip()):
                        # Only as a step env value or a reusable-workflow secret.
                        self.assertRegex(line.strip(), r"^(STELLAR_ACCOUNT|STELLAR_SIGN_WITH_KEY|signing_key): \$\{\{ " + re.escape(SEED) + r" \}\}$")

    def test_npm_publish_only_publishes_a_packed_tarball(self):
        jobs = release_jobs()
        minting = {n: j for n, j in jobs.items() if "id-token: write" in j and "npm publish" in j}
        self.assertEqual(list(minting), ["npm-publish"])
        job = minting["npm-publish"]
        self.assertRegex(job, r"(?m)^    environment: npm$")
        self.assertIn("--ignore-scripts", job)
        self.assertNotIn("actions/checkout", job)
        for marker in BUILDS:
            with self.subTest(marker=marker):
                self.assertNotIn(marker, job)
        self.assertNotIn("id-token", jobs["npm-pack"])
        # A tarball publish records no gitHead of its own, and the Nido
        # dispatch plan requires one naming the release commit.
        self.assertIn('npm pkg set gitHead="$(git rev-parse HEAD)"', jobs["npm-pack"])


NPM = shutil.which("npm")


def deploy_pr_add_paths():
    """The paths release.yml's deploy-stack commits into its deploy PR."""
    block = re.search(r"^ *- name: Open the deploy\(testnet\) manifest PR\n(.*?)^ *body:", release_yml(), re.S | re.M).group(1)
    return re.search(r"add-paths: \|\n((?: +\S+\n)+)", block).group(1).split()


@unittest.skipUnless(GIT_CLIFF or os.environ.get("PERCH_REQUIRE_GIT_CLIFF"), "git-cliff not on PATH")
@unittest.skipUnless(NPM or os.environ.get("PERCH_REQUIRE_GIT_CLIFF"), "npm not on PATH")
class NpmBump(unittest.TestCase):
    """scripts/npm-release.sh bump, as release-pr runs it for every package
    and deploy-stack runs it for perch-artifacts over its deploy commit."""

    def setUp(self):
        self.repo = Repo()
        shutil.copy(os.path.join(REPO, "cliff.toml"), os.path.join(self.repo.dir, "cliff.toml"))
        self.repo.write("packages/perch-artifacts/package.json", json.dumps(
            {"name": "@stellar-registry/perch-artifacts", "version": "0.1.0"}, indent=2) + "\n")
        self.repo.write("packages/perch-artifacts/package-lock.json", json.dumps({
            "name": "@stellar-registry/perch-artifacts", "version": "0.1.0", "lockfileVersion": 3,
            "packages": {"": {"name": "@stellar-registry/perch-artifacts", "version": "0.1.0"}},
        }, indent=2) + "\n")
        self.repo.write("deployments/testnet.json", '{"a": 1}\n')
        self.repo.write("crates/perch-account/wasm/dep.sha256", "aa\n")
        self.repo.commit("feat(perch-artifacts): first release")

    def tearDown(self):
        self.repo.cleanup()

    def bump(self):
        return run([NPM_RELEASE, "bump", "perch-artifacts"], self.repo.dir).stdout.strip()

    def version(self, name):
        with open(os.path.join(self.repo.dir, "packages", "perch-artifacts", name)) as f:
            return json.load(f)["version"]

    def test_an_untagged_package_is_not_bumped(self):
        self.assertEqual(self.bump(), "")

    def test_a_deploy_commit_bumps_perch_artifacts(self):
        # deploy-stack's step: commit the new record, bump, uncommit.
        run(["git", "tag", "perch-artifacts-v0.1.0"], self.repo.dir)
        self.assertEqual(self.bump(), "")
        self.repo.write("deployments/testnet.json", '{"a": 2}\n')
        self.repo.commit("deploy(testnet): recovery stack @ " + "ab" * 20)
        self.assertEqual(self.bump(), "0.1.0 0.1.1")
        self.assertEqual(self.version("package.json"), "0.1.1")
        self.assertEqual(self.version("package-lock.json"), "0.1.1")
        with open(os.path.join(self.repo.dir, "packages", "perch-artifacts", "CHANGELOG.md")) as f:
            changelog = f.read()
        self.assertIn("## [0.1.1]", changelog)
        self.assertIn("*(testnet)* Recovery stack @ " + "ab" * 20, changelog)
        # Every file the bump writes rides in the deploy PR's one commit.
        changed = run(["git", "status", "--porcelain", "--untracked-files=all"], self.repo.dir).stdout.split()[1::2]
        for path in changed:
            self.assertTrue(any(path == p or path.startswith(p + "/") for p in deploy_pr_add_paths()), path)
        # The bumped version is current: no further bump until a later change.
        self.repo.commit("chore: take the bump")
        self.assertEqual(self.bump(), "")

    def test_a_pin_only_change_is_no_release(self):
        run(["git", "tag", "perch-artifacts-v0.1.0"], self.repo.dir)
        self.repo.write("crates/perch-account/wasm/dep.sha256", "bb\n")
        self.repo.commit("deploy(testnet): recovery stack")
        self.assertEqual(self.bump(), "")

    def test_a_version_already_tagged_is_not_proposed_again(self):
        # detect-releases tagged the merged bump before release-pr ran.
        run(["git", "tag", "perch-artifacts-v0.1.0"], self.repo.dir)
        self.repo.write("deployments/testnet.json", '{"a": 2}\n')
        self.repo.commit("deploy(testnet): recovery stack")
        self.assertEqual(self.bump(), "0.1.0 0.1.1")
        self.repo.commit("chore: take the bump")
        run(["git", "tag", "perch-artifacts-v0.1.1"], self.repo.dir)
        self.assertEqual(self.bump(), "")


class Registry:
    """A local npm registry serving perch-artifacts version documents and
    tarballs, for scripts/nido-dispatch-plan.sh."""

    def __init__(self):
        self.files = {}
        files = self.files

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                body = files.get(self.path)
                self.send_response(200 if body is not None else 404)
                self.end_headers()
                if body is not None:
                    self.wfile.write(body)

            def log_message(self, *args):
                pass

        self.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def publish(self, version, git_head, files, provenance=True, integrity=None):
        buf = io.BytesIO()
        with tarfile.open(fileobj=buf, mode="w:gz") as tar:
            for path, data in files.items():
                info = tarfile.TarInfo(f"package/{path}")
                info.size = len(data)
                tar.addfile(info, io.BytesIO(data))
        tgz = buf.getvalue()
        tarball = f"/@stellar-registry/perch-artifacts/-/perch-artifacts-{version}.tgz"
        dist = {
            "tarball": self.url + tarball,
            "integrity": integrity or "sha512-" + base64.b64encode(hashlib.sha512(tgz).digest()).decode(),
        }
        if provenance:
            dist["attestations"] = {"provenance": {"predicateType": "https://slsa.dev/provenance/v1"}}
        self.files[tarball] = tgz
        self.files[f"/@stellar-registry%2fperch-artifacts/{version}"] = json.dumps(
            {"version": version, "gitHead": git_head, "dist": dist}).encode()

    def close(self):
        self.server.shutdown()
        self.server.server_close()


def manifest(network, n):
    return (json.dumps({"schema": 1, "network": network, "n": n, "source": {"commit": "cd" * 20}}, indent=2) + "\n").encode()


class NidoDispatchPlan(unittest.TestCase):
    """scripts/nido-dispatch-plan.sh: which manifests a perch-artifacts
    release announces to Nido, and the package it must find first."""

    def setUp(self):
        self.repo = Repo()
        self.registry = Registry()
        self.repo.write("deployments/testnet.json", manifest("testnet", 1).decode())
        self.repo.commit("feat: first deployment")
        self.tag("0.1.0")

    def tearDown(self):
        self.registry.close()
        self.repo.cleanup()

    def tag(self, version):
        run(["git", "tag", f"perch-artifacts-v{version}"], self.repo.dir)
        return self.head()

    def head(self):
        return run(["git", "rev-parse", "HEAD"], self.repo.dir).stdout.strip()

    def publish(self, version, **over):
        files = {}
        for path in ("deployments/testnet.json", "deployments/mainnet.json"):
            full = os.path.join(self.repo.dir, path)
            if os.path.exists(full):
                with open(full, "rb") as f:
                    files[path] = f.read()
        args = {"git_head": self.head(), "files": files, **over}
        self.registry.publish(version, **args)

    def plan(self, *args, check=True):
        env = {**os.environ, "REPOSITORY": "stellar-registry/perch", "RUN_URL": "https://example.com/run/1",
               "NPM_REGISTRY": self.registry.url, "REGISTRY_TRIES": "1"}
        out = subprocess.run([NIDO_PLAN, *args], cwd=self.repo.dir, capture_output=True, text=True, env=env)
        if check:
            self.assertEqual(out.returncode, 0, out.stderr)
            return json.loads(out.stdout)
        self.assertNotEqual(out.returncode, 0, out.stdout)
        return out.stderr

    def redeploy(self, n, version):
        self.repo.write("deployments/testnet.json", manifest("testnet", n).decode())
        self.repo.commit("deploy(testnet): recovery stack")
        return self.tag(version)

    def test_a_release_with_a_new_manifest_dispatches_its_version(self):
        sha = self.redeploy(2, "0.1.1")
        self.publish("0.1.1")
        [target] = self.plan("release", "perch-artifacts-v0.1.1")
        self.assertEqual(target["network"], "testnet")
        self.assertEqual(target["payload"], {
            "schema": 1,
            "repository": "stellar-registry/perch",
            "network": "testnet",
            "perch_sha": sha,
            "manifest_path": "deployments/testnet.json",
            "manifest_sha256": hashlib.sha256(manifest("testnet", 2)).hexdigest(),
            "source_commit": "cd" * 20,
            "run_url": "https://example.com/run/1",
            "artifacts_package": "@stellar-registry/perch-artifacts",
            "artifacts_version": "0.1.1",
        })

    def test_a_release_with_no_manifest_change_dispatches_nothing(self):
        # Without asking the registry: nothing is announced.
        self.repo.write("circuits/x.nr", "fn main() {}\n")
        self.repo.commit("fix: a circuit")
        self.tag("0.1.1")
        self.assertEqual(self.plan("release", "perch-artifacts-v0.1.1"), [])

    def test_the_first_release_dispatches_every_manifest(self):
        self.publish("0.1.0")
        self.assertEqual([t["network"] for t in self.plan("release", "perch-artifacts-v0.1.0")], ["testnet"])

    def test_only_the_networks_whose_manifest_changed(self):
        self.repo.write("deployments/mainnet.json", manifest("mainnet", 1).decode())
        self.repo.commit("deploy(mainnet): recovery stack")
        self.tag("0.2.0")
        self.publish("0.2.0")
        self.assertEqual([t["network"] for t in self.plan("release", "perch-artifacts-v0.2.0")], ["mainnet"])

    def test_resend_names_the_latest_release(self):
        self.redeploy(2, "0.1.1")
        self.publish("0.1.1")
        sha = self.redeploy(3, "0.1.2")
        self.publish("0.1.2")
        [target] = self.plan("resend", "testnet")
        self.assertEqual((target["payload"]["artifacts_version"], target["payload"]["perch_sha"]), ("0.1.2", sha))

    def test_an_unpublished_version_is_refused(self):
        self.redeploy(2, "0.1.1")
        self.assertIn("not on the registry", self.plan("release", "perch-artifacts-v0.1.1", check=False))

    def test_a_package_from_another_commit_is_refused(self):
        self.redeploy(2, "0.1.1")
        self.publish("0.1.1", git_head="ef" * 20)
        self.assertIn("was published from", self.plan("release", "perch-artifacts-v0.1.1", check=False))

    def test_a_package_without_provenance_is_refused(self):
        self.redeploy(2, "0.1.1")
        self.publish("0.1.1", provenance=False)
        self.assertIn("no provenance", self.plan("release", "perch-artifacts-v0.1.1", check=False))

    def test_a_tarball_not_matching_its_integrity_is_refused(self):
        self.redeploy(2, "0.1.1")
        self.publish("0.1.1", integrity="sha512-AAAA")
        self.assertIn("integrity", self.plan("release", "perch-artifacts-v0.1.1", check=False))

    def test_a_package_shipping_other_manifest_bytes_is_refused(self):
        self.redeploy(2, "0.1.1")
        self.publish("0.1.1", files={"deployments/testnet.json": manifest("testnet", 1)})
        self.assertIn("does not ship deployments/testnet.json", self.plan("release", "perch-artifacts-v0.1.1", check=False))

    def test_a_file_that_is_not_the_networks_manifest_is_refused(self):
        self.repo.write("deployments/testnet.json", manifest("mainnet", 2).decode())
        self.repo.commit("deploy(testnet): wrong network")
        self.tag("0.1.1")
        self.publish("0.1.1")
        self.assertIn("is not a schema-1 testnet manifest", self.plan("release", "perch-artifacts-v0.1.1", check=False))

    def test_only_a_perch_artifacts_tag(self):
        run(["git", "tag", "perch-contracts-v0.1.0"], self.repo.dir)
        self.assertIn("is not a perch-artifacts-v<semver> tag", self.plan("release", "perch-contracts-v0.1.0", check=False))
        self.assertIn("unknown network", self.plan("resend", "devnet", check=False))


class SourceDirty(unittest.TestCase):
    INPUTS = [
        "crates/a/src/lib.rs",
        "vendor/ultrahonk-soroban-verifier/src/lib.rs",
        "circuits/recovery/src/main.nr",
        "scripts/build.sh",
        ".cargo/config.toml",
        "Cargo.toml",
        "Cargo.lock",
        "rust-toolchain.toml",
    ]

    def setUp(self):
        self.repo = Repo()
        for path in self.INPUTS + ["docs/notes.md"]:
            self.repo.write(path, "original\n")
        self.repo.write(".gitignore", "target/\ncrates/a/wasm/\n")
        self.repo.commit("base")

    def tearDown(self):
        self.repo.cleanup()

    def dirty(self):
        return run([SOURCE_DIRTY, self.repo.dir], self.repo.dir).stdout.strip()

    def test_a_clean_checkout_is_clean(self):
        self.assertEqual(self.dirty(), "false")

    def test_an_edit_to_any_build_input_is_dirty(self):
        for path in self.INPUTS:
            self.repo.write(path, "edited\n")
            self.assertEqual(self.dirty(), "true", path)
            run(["git", "checkout", "--", path], self.repo.dir)
            self.assertEqual(self.dirty(), "false", path)

    def test_an_untracked_file_in_a_build_input_is_dirty(self):
        self.repo.write("vendor/ultrahonk-soroban-verifier/src/extra.rs", "new\n")
        self.assertEqual(self.dirty(), "true")

    def test_docs_and_ignored_files_are_not(self):
        self.repo.write("docs/notes.md", "edited\n")
        self.repo.write("target/stack/build.json", "{}\n")
        self.repo.write("crates/a/wasm/pin.wasm", "bytes\n")
        self.assertEqual(self.dirty(), "false")

    def test_the_committed_hash_pins_are_not(self):
        # build-stack.sh rewrites them for the build it records.
        self.repo.write("crates/b/wasm/dep.sha256", "aa\n")
        self.repo.commit("pins")
        self.repo.write("crates/b/wasm/dep.sha256", "bb\n")
        self.repo.write("crates/b/wasm/new.sha256", "cc\n")
        self.assertEqual(self.dirty(), "false")
        self.repo.write("crates/b/wasm/README.md", "edited\n")
        self.assertEqual(self.dirty(), "true")

    def test_the_registry_path_is_source(self):
        # Compiled in, and no build rewrites it.
        self.repo.write("crates/b/wasm/registry.path", "root-deployer G\nroot-salt s\npath a\n")
        self.repo.commit("path")
        self.repo.write("crates/b/wasm/registry.path", "root-deployer G\nroot-salt s\npath b\n")
        self.assertEqual(self.dirty(), "true")


class CheckPins(unittest.TestCase):
    TESTNET = "Test SDF Network ; September 2015"
    MANIFEST = {
        "network": "testnet",
        "network_passphrase": TESTNET,
        "registry": {"id": "CREG"},
        "contracts": {"dep": {"sha256": "aa"}, "acct": {"sha256": "bb"}},
    }
    PATH = "root-deployer GROOT\nroot-salt s\npath perch/constructorless\n"

    def setUp(self):
        self.repo = Repo()
        self.repo.write("deployments/testnet.json", json.dumps(self.MANIFEST))
        self.repo.write("crates/a/wasm/registry.path", self.PATH)
        self.repo.write("crates/a/wasm/dep.sha256", "aa\n")
        self.repo.write("crates/b/wasm/registry.path", self.PATH)
        self.repo.write("crates/b/wasm/acct.sha256", "bb\n")
        os.makedirs(os.path.join(self.repo.dir, "scripts"))
        for script in (CHECK_PINS, MANIFEST_REGISTRY):
            shutil.copy(script, os.path.join(self.repo.dir, "scripts"))
        # Stands in for `perch-derive-id registry <passphrase>`, the account's
        # own derivation: prints the registry in `derived` for the testnet
        # passphrase, a different one for any other.
        self.repo.write("derive-id", textwrap.dedent(f"""\
            #!/bin/sh
            [ "$1" = registry ] || exit 2
            if [ "$2" = "{self.TESTNET}" ]; then cat "$(dirname "$0")/derived"; else echo COTHERNET; fi
            """))
        os.chmod(os.path.join(self.repo.dir, "derive-id"), 0o755)
        self.derives("CREG")

    def tearDown(self):
        self.repo.cleanup()

    def derives(self, registry):
        self.repo.write("derived", registry + "\n")

    def pending(self, recorded, derived):
        self.repo.write("deployments/pending-registry.json", json.dumps(
            {"testnet": {"recorded": recorded, "derived": derived, "reason": "r"}}))

    def check(self):
        env = dict(os.environ, PERCH_DERIVE_ID=os.path.join(self.repo.dir, "derive-id"))
        env.pop("GITHUB_ACTIONS", None)
        return subprocess.run(["scripts/check-pins.sh"], cwd=self.repo.dir, env=env,
                              capture_output=True, text=True)

    def test_pins_matching_the_manifest_pass(self):
        out = self.check()
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertIn("3 pins match testnet.json", out.stderr)
        self.assertNotIn("pending", out.stderr)

    def test_a_drifted_hash_fails(self):
        self.repo.write("crates/b/wasm/acct.sha256", "cc\n")
        out = self.check()
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("crates/b/wasm/acct.sha256 is 'cc'", out.stderr)

    def test_a_registry_the_path_does_not_derive_fails(self):
        self.derives("CNEW")
        out = self.check()
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("records registry CREG, this source derives CNEW", out.stderr)

    def test_a_declared_migration_passes_and_says_so(self):
        self.derives("CNEW")
        self.pending("CREG", "CNEW")
        out = self.check()
        self.assertEqual(out.returncode, 0, out.stderr)
        self.assertIn("pending registry migration: testnet.json records CREG, the registry path derives CNEW",
                      out.stderr)
        # The hashes are still checked.
        self.repo.write("crates/b/wasm/acct.sha256", "cc\n")
        self.assertNotEqual(self.check().returncode, 0)

    def test_a_declaration_for_another_pair_fails(self):
        self.derives("CNEW")
        for recorded, derived in (("COLD", "CNEW"), ("CREG", "COTHER")):
            self.pending(recorded, derived)
            self.assertNotEqual(self.check().returncode, 0, (recorded, derived))

    def test_a_declaration_the_manifest_caught_up_with_fails(self):
        self.pending("COLD", "CREG")
        out = self.check()
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("delete the testnet entry from deployments/pending-registry.json", out.stderr)

    def test_differing_registry_paths_fail(self):
        self.repo.write("crates/b/wasm/registry.path", self.PATH.replace("perch/", "other/"))
        out = self.check()
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("registry.path differs", out.stderr)

    def test_a_pin_the_manifest_lacks_fails(self):
        self.repo.write("crates/a/wasm/gone.sha256", "dd\n")
        out = self.check()
        self.assertNotEqual(out.returncode, 0)
        self.assertIn("<not in the manifest>", out.stderr)

    def test_no_pins_fails(self):
        for d in ("a", "b"):
            shutil.rmtree(os.path.join(self.repo.dir, "crates", d))
        self.assertNotEqual(self.check().returncode, 0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
