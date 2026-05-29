// SPDX-License-Identifier: Apache-2.0
//
// Release Automation Script
// Coordinates building binaries, PATH registration, committing/pushing, publishing, and releasing.
//
// Usage:
//   bun scripts/release.ts --version 1.5.9
//   bun scripts/release.ts --version 1.5.9 --dry-run

import {join} from 'node:path';
import {readFileSync, writeFileSync, existsSync} from 'node:fs';

const ROOT = join(import.meta.dir, '..');

const args = Bun.argv.slice(2);
const dryRun = args.includes('--dry-run');
const versionIdx = args.indexOf('--version');
if (versionIdx === -1 || !args[versionIdx + 1]) {
  console.error("Error: Please specify the version to release using --version <version> (e.g. --version 1.5.9)");
  process.exit(1);
}
const VERSION = args[versionIdx + 1];

function log(step: string, message: string) {
  console.log(`\x1b[36m[${step}]\x1b[0m ${message}`);
}

function logSuccess(step: string, message: string) {
  console.log(`\x1b[32m[${step}] ✓ ${message}\x1b[0m`);
}

function logError(step: string, message: string) {
  console.error(`\x1b[31m[${step}] ✗ ${message}\x1b[0m`);
}

function runCmd(cmd: string[], cwd: string = ROOT): boolean {
  log("EXEC", `${cmd.join(' ')} (in ${cwd})`);
  if (dryRun) {
    log("DRY-RUN", "Skipping actual execution.");
    return true;
  }
  const p = Bun.spawnSync(cmd, {cwd, stdout: 'inherit', stderr: 'inherit'});
  return p.exitCode === 0;
}

// 1. Resolve and Load GitHub Token
let token = process.env['GITHUB_TOKEN'];
if (!token) {
  const vpsMirrorNpmrc = 'C:\\Users\\yohan\\vps-mirror\\.npmrc';
  if (existsSync(vpsMirrorNpmrc)) {
    const npmrcContent = readFileSync(vpsMirrorNpmrc, 'utf8');
    const match = npmrcContent.match(/\/\/npm\.pkg\.github\.com\/:_authToken=(ghp_[A-Za-z0-9]+)/);
    if (match && match[1]) {
      token = match[1];
      process.env['GITHUB_TOKEN'] = token;
      log("AUTH", "Loaded GITHUB_TOKEN from vps-mirror/.npmrc");
    }
  }
}

if (!token) {
  logError("AUTH", "No GITHUB_TOKEN set or found in vps-mirror/.npmrc. Release will fail.");
  process.exit(1);
}

// 2. Bump Versions in package.json files
log("VERSION", `Bumping versions to ${VERSION}...`);
const packagesToBump = [
  {path: join(ROOT, 'packages/skills/package.json'), name: 'skills'}
];

for (const pkg of packagesToBump) {
  if (existsSync(pkg.path)) {
    const pkgObj = JSON.parse(readFileSync(pkg.path, 'utf8'));
    const oldVersion = pkgObj.version;
    pkgObj.version = VERSION;
    if (!dryRun) {
      writeFileSync(pkg.path, JSON.stringify(pkgObj, null, 2) + '\n');
    }
    logSuccess("VERSION", `Bumped ${pkg.name} package.json version from ${oldVersion} to ${VERSION}`);
  } else {
    logError("VERSION", `Could not find file: ${pkg.path}`);
    process.exit(1);
  }
}

// 3. Compile Standalone Binaries
log("BUILD", "Compiling skills CLI binary...");
if (!runCmd(['bun', 'build', '--compile', './packages/skills/src/cli.ts', '--outfile', './bin/skills.exe'])) {
  logError("BUILD", "Failed to compile skills CLI binary.");
  process.exit(1);
}

logSuccess("BUILD", "Standalone binaries compiled successfully.");

// 4. Update PATH
log("PATH", "Registering bin/ in environment PATH...");
const psCommand = `
$binPath = "c:\\src\\aphrody-ts\\bin"
$currentPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$binPath*") {
    [System.Environment]::SetEnvironmentVariable("PATH", $currentPath + ";" + $binPath, "User")
    $env:Path += ";$binPath"
    Write-Host "PATH updated successfully."
} else {
    Write-Host "PATH already contains bin/."
}
`;
if (!runCmd(['powershell', '-Command', psCommand])) {
  logError("PATH", "Failed to update global PATH environment variable.");
} else {
  logSuccess("PATH", "PATH check/update complete.");
}

// 5. Git Commit & Push for packages/skills
log("GIT-SKILLS", "Handling git repository for packages/skills...");
const skillsCwd = join(ROOT, 'packages/skills');
if (existsSync(skillsCwd)) {
  runCmd(['git', 'add', '.'], skillsCwd);
  runCmd(['git', 'commit', '-m', `chore: release version ${VERSION}`], skillsCwd);
  if (!runCmd(['git', 'push', 'origin', 'main'], skillsCwd)) {
    logError("GIT-SKILLS", "Failed to push packages/skills to GitHub.");
  } else {
    logSuccess("GIT-SKILLS", "packages/skills pushed successfully.");
  }
}

// 6. Git Commit & Push for root repo
log("GIT-ROOT", "Handling git repository for aphrody-ts...");
runCmd(['git', 'add', '.'], ROOT);
runCmd(['git', 'commit', '-m', `chore: release version v${VERSION}`], ROOT);
if (!runCmd(['git', 'push', 'origin', 'main'], ROOT)) {
  logError("GIT-ROOT", "Failed to push aphrody-ts to GitHub.");
  process.exit(1);
}
logSuccess("GIT-ROOT", "aphrody-ts main branch pushed successfully.");

// 7. Publish Scoped Packages
log("PUBLISH", "Publishing packages to GitHub Packages registry...");
if (!runCmd(['bun', 'scripts/publish-github-packages.ts', '--publish'])) {
  logError("PUBLISH", "Failed to publish packages.");
  process.exit(1);
}
logSuccess("PUBLISH", "Packages published successfully.");

// 8. Create Git Tag & Push
log("TAG", `Tagging repository as v${VERSION}...`);
runCmd(['git', 'tag', `v${VERSION}`], ROOT);
if (!runCmd(['git', 'push', 'origin', `v${VERSION}`], ROOT)) {
  logError("TAG", "Failed to push git tag.");
  process.exit(1);
}
logSuccess("TAG", "Git tag created and pushed.");

// 9. Create GitHub Release & Upload Assets
log("RELEASE", "Creating draft release and uploading binaries...");
if (!runCmd([
  'gh', 'release', 'create', `v${VERSION}`,
  './bin/skills.exe',
  '--title', `Release v${VERSION}`,
  '--notes', `Release containing compiled skills CLI binary.`
])) {
  logError("RELEASE", "Failed to create release / upload assets.");
  process.exit(1);
}

log("RELEASE", "Publishing release (draft=false)...");
if (!runCmd(['gh', 'release', 'edit', `v${VERSION}`, '--draft=false'])) {
  logError("RELEASE", "Failed to publish release.");
  process.exit(1);
}

logSuccess("RELEASE", `Release v${VERSION} successfully created and published!`);
console.log(`\n\x1b[32;1m=== RELEASE SUCCESSFUL FOR VERSION ${VERSION} ===\x1b[0m\n`);
