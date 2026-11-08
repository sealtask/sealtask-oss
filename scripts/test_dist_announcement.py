"""Exercise generated announcement commands with a checkout artifact collision."""

import glob
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/release.yml"


class AnnouncementTests(unittest.TestCase):
    def test_uploads_only_downloaded_assets(self):
        announcement = WORKFLOW.read_text().split("  announce:\n", 1)[1]
        announcement = announcement.split("\n  custom-verify-release:", 1)[0]
        download_path = re.search(r"^          path: (.+)$", announcement, re.M).group(1)
        subject_path = re.search(r"subject-path: \|\n            (.+)", announcement).group(1)

        def run_script(step):
            section = announcement.split(f"      - name: {step}\n", 1)[1]
            section = section.split("      - name:", 1)[0]
            script = textwrap.dedent(section.split("        run: |\n", 1)[1])
            return script.replace("${{ needs.plan.outputs.tag }}", "v0.5.1")

        for state in ("absent", "draft", "immutable"):
            with self.subTest(state=state), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                checkout = root / "checkout"
                tracked = checkout / "artifacts/strong-box-wasm"
                tracked.mkdir(parents=True)
                sentinel = tracked / "build-manifest.json"
                sentinel.write_text("tracked source")
                runner_temp = root / "runner-temp"
                runner_temp.mkdir()
                assets = Path(download_path.replace("${{ runner.temp }}", str(runner_temp)))
                if not assets.is_absolute():
                    assets = checkout / assets
                assets.mkdir(parents=True, exist_ok=True)
                expected = {assets / name for name in ("sealtask-installer.sh", "dist-manifest.json")}
                for path in expected:
                    path.write_text("release asset")
                (assets / "plan-dist-manifest.json").write_text("granular manifest")
                commands = root / "commands.jsonl"
                bin_dir = root / "bin"
                bin_dir.mkdir()
                gh = bin_dir / "gh"
                gh.write_text(textwrap.dedent("""\
                    #!/usr/bin/env python3
                    import json, os, sys
                    args = sys.argv[1:]
                    if args[:2] == ['release', 'view']:
                        state = os.environ['RELEASE_STATE']
                        if state == 'absent':
                            sys.exit(1)
                        print(json.dumps({'tagName': 'v0.5.1',
                                          'isDraft': state == 'draft',
                                          'isImmutable': state == 'immutable'}))
                    else:
                        with open(os.environ['COMMAND_LOG'], 'a') as log:
                            log.write(json.dumps(args) + '\\n')
                    """))
                gh.chmod(0o755)
                env = dict(os.environ, PATH=f"{bin_dir}:{os.environ['PATH']}",
                           RUNNER_TEMP=str(runner_temp), RELEASE_STATE=state,
                           COMMAND_LOG=str(commands), ANNOUNCEMENT_BODY="notes",
                           ANNOUNCEMENT_TITLE="release", PRERELEASE_FLAG="")
                for step in ("Cleanup", "Create GitHub Release"):
                    result = subprocess.run(["bash", "-e", "-c", run_script(step)],
                                            cwd=checkout, env=env, capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(sentinel.read_text(), "tracked source")
                subjects = subject_path.replace("${{ runner.temp }}", str(runner_temp))
                if not Path(subjects).is_absolute():
                    subjects = str(checkout / subjects)
                self.assertEqual(set(map(Path, glob.glob(subjects))), expected)
                calls = [json.loads(line) for line in commands.read_text().splitlines()] if commands.exists() else []
                uploads = [call for call in calls if call[:2] in (["release", "create"], ["release", "upload"])]
                if state == "immutable":
                    self.assertEqual(calls, [])
                    continue
                self.assertEqual(len(uploads), 1)
                passed_assets = {Path(arg) for arg in uploads[0] if str(assets) in arg or arg.startswith("artifacts/")}
                self.assertEqual(passed_assets, expected)
                self.assertTrue(all(path.is_file() for path in passed_assets))


if __name__ == "__main__":
    unittest.main()
