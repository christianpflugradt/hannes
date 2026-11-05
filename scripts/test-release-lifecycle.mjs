// Exercise the real release lifecycle against a disposable local bare remote.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createRequire } from "node:module";
import { Writable } from "node:stream";
import semanticRelease from "semantic-release";
import config from "../release.config.mjs";
import { verifyRelease } from "./semantic-release-plugin.mjs";

const require = createRequire(import.meta.url);
const source = resolve(import.meta.dirname, "..");

function testGuard() {
  assert.throws(() => verifyRelease({}, {
    env: { EXPECTED_RELEASE_VERSION: "1.0.0" }, nextRelease: { version: "1.1.0" },
  }), /version changed/);
}

async function testLifecycle() {
  const temporary = mkdtempSync(join(tmpdir(), "hannes-release-test-"));
  const cwd = join(temporary, "repository");
  const remote = join(temporary, "remote.git");
  mkdirSync(cwd);
  const env = { ...process.env, EXPECTED_RELEASE_VERSION: "1.0.0" };
  for (const key of Object.keys(env)) {
    if (key.startsWith("GITHUB_") || ["CI", "GH_TOKEN", "GITLAB_CI"].includes(key)) delete env[key];
  }
  const git = (...args) => execFileSync("git", args, { cwd, env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  try {
    git("init", "--bare", "--initial-branch=main", remote);
    git("init", "--initial-branch=main");
    git("config", "user.name", "Release Fixture");
    git("config", "user.email", "fixture@example.invalid");
    git("config", "commit.gpgsign", "false");
    git("config", "tag.gpgsign", "false");
    git("remote", "add", "origin", remote);
    for (const file of ["README.md", "LICENSE", "Cargo.toml", "Cargo.lock"]) {
      copyFileSync(join(source, file), join(cwd, file));
    }
    mkdirSync(join(cwd, "scripts"));
    for (const file of ["verify-release.py", "set-release-version.py"]) {
      copyFileSync(join(source, "scripts", file), join(cwd, "scripts", file));
    }
    writeFileSync(join(cwd, ".gitignore"), "dist/\n");
    git("add", ".");
    git("commit", "-m", "feat: initialize fixture");
    git("push", "--set-upstream", "origin", "main");
    env.GITHUB_SHA = git("rev-parse", "HEAD");
    execFileSync("python3", ["-c", `
import hashlib, io, json, os, pathlib, tarfile
root=pathlib.Path('.')
dist=root/'dist'; (dist/'metadata').mkdir(parents=True)
targets=['aarch64-apple-darwin','x86_64-apple-darwin','x86_64-unknown-linux-gnu']
for target in targets:
    name=f'hannes-v1.0.0-{target}'
    binary=f'fixture executable for {target}'.encode()
    archive=dist/f'{name}.tar.gz'
    with tarfile.open(archive,'w:gz') as output:
        for file,data in [('hannes',binary),('README.md',pathlib.Path('README.md').read_bytes()),('LICENSE',pathlib.Path('LICENSE').read_bytes())]:
            member=tarfile.TarInfo(f'{name}/{file}');member.size=len(data);member.mode=0o755 if file=='hannes' else 0o644
            output.addfile(member,io.BytesIO(data))
    (dist/'metadata'/f'{target}.json').write_text(json.dumps(dict(version='1.0.0',source_sha=os.environ['GITHUB_SHA'],target=target,archive_sha256=hashlib.sha256(archive.read_bytes()).hexdigest(),binary_sha256=hashlib.sha256(binary).hexdigest())))
(dist/'SHA256SUMS').write_text(''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\\n' for p in sorted(dist.glob('*.tar.gz'))))
`], { cwd, env });
    const notesPlugin = join(temporary, "notes.mjs");
    writeFileSync(notesPlugin, 'export function generateNotes() { return "Fixture release notes"; }\n');
    const plugins = config.plugins.slice(0, -1).map(plugin => {
      const [name, options] = Array.isArray(plugin) ? plugin : [plugin, {}];
      const path = name === "@semantic-release/release-notes-generator" ? notesPlugin
        : name.startsWith(".") ? resolve(source, name) : require.resolve(name);
      return [path, options];
    });
    const quiet = new Writable({ write(_chunk, _encoding, done) { done(); } });
    const options = { ...config, repositoryUrl: remote, ci: false, plugins };
    const context = { cwd, env, stdout: quiet, stderr: quiet };
    const original = readFileSync(join(cwd, "Cargo.toml"), "utf8");
    const preview = await semanticRelease({ ...options, dryRun: true }, context);
    assert.equal(preview.nextRelease.version, "1.0.0");
    assert.equal(git("tag"), "");
    assert.equal(readFileSync(join(cwd, "Cargo.toml"), "utf8"), original);
    // A rejected run must not create a release tag or a generated version commit.
    await assert.rejects(semanticRelease(options, { ...context, env: { ...env, EXPECTED_RELEASE_VERSION: "9.9.9" } }));
    assert.equal(git("tag"), "");
    assert.equal(git("rev-parse", "HEAD"), env.GITHUB_SHA);
    const result = await semanticRelease(options, context);
    assert.equal(result.nextRelease.version, "1.0.0");
    assert.match(readFileSync(join(cwd, "Cargo.toml"), "utf8"), /version = "1.0.0"/);
    assert.match(readFileSync(join(cwd, "Cargo.lock"), "utf8"), /name = "hannes"\nversion = "1.0.0"/);
    assert.match(git("log", "-1", "--format=%s"), /^build\(release\): 1\.0\.0 \[skip ci\]$/);
    assert.equal(git("rev-parse", "v1.0.0"), git("rev-parse", "HEAD"));
    assert.match(git("ls-remote", "origin", "refs/tags/v1.0.0"), /refs\/tags\/v1\.0\.0/);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

// semantic-release wraps stdout; run outside node:test's binary IPC stream.
testGuard();
await testLifecycle();
console.log("Release lifecycle passed: inert preview, version guard, artifact validation, generated commit and local tag.");
