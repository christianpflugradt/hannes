"""Unwrap only Gemini CLI's documented JSON response envelope; Hannes validates the verdict."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

LIMIT = 65536


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate envelope key")
        result[key] = value
    return result


def unwrap(raw):
    if len(raw) > LIMIT:
        raise ValueError("Gemini envelope exceeds capture limit")
    envelope = json.loads(raw, object_pairs_hook=unique_object)
    if not isinstance(envelope, dict) or "error" in envelope:
        raise ValueError("Gemini returned no reliable response")
    response = envelope.get("response")
    if not isinstance(response, str) or not response.strip():
        raise ValueError("Gemini response must be a nonempty string")
    return response


def run(prompt):
    executable = shutil.which("gemini")
    if not executable or not os.environ.get("GEMINI_API_KEY"):
        raise ValueError("Gemini executable and GEMINI_API_KEY are required")
    # An isolated cwd/home prevents repository/user memory, extensions, MCP and
    # hooks from becoming extra review inputs. This is not an OS sandbox.
    with tempfile.TemporaryDirectory(prefix="hannes-gemini-") as directory:
        root = Path(directory)
        settings = root / "settings.json"
        settings.write_text(json.dumps({"security": {"auth": {"selectedType": "gemini-api-key"}},
                                       "telemetry": {"enabled": False},
                                       "privacy": {"usageStatisticsEnabled": False}}))
        policy = root / "deny-tools.toml"
        policy.write_text('[[rule]]\ntoolName = "*"\ndecision = "deny"\npriority = 999\n')
        env = {"PATH": os.environ.get("PATH", ""), "GEMINI_API_KEY": os.environ["GEMINI_API_KEY"],
               "GEMINI_CLI_HOME": directory, "GEMINI_CLI_SYSTEM_SETTINGS_PATH": str(settings),
               "GEMINI_CLI_SYSTEM_DEFAULTS_PATH": str(settings)}
        with tempfile.TemporaryFile() as output:
            result = subprocess.run([executable, "--prompt", "Evaluate only the supplied Hannes input. Return only the required JSON verdict; do not use tools.",
                                     "--model", os.environ["HANNES_MODEL"], "--output-format", "json",
                                     "--policy", str(policy)], input=prompt, stdout=output,
                                    stderr=subprocess.DEVNULL, cwd=root, env=env)
            if result.returncode:
                raise ValueError("Gemini CLI exited unsuccessfully")
            output.seek(0)
            return unwrap(output.read(LIMIT + 1))


if __name__ == "__main__":
    try:
        sys.stdout.write(run(sys.stdin.buffer.read()))
    except (OSError, ValueError, KeyError, subprocess.SubprocessError):
        sys.exit("Gemini review could not produce a reliable response; raw output is suppressed")
