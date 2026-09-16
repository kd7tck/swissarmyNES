// Reconstruct the patched dependency from a checksum-pinned upstream package.
// Only this script and the source patch belong in version control.
import {createHash} from 'node:crypto';
import {existsSync} from 'node:fs';
import {mkdir, mkdtemp, readFile, readdir, realpath, rename, rm, writeFile, lstat} from 'node:fs/promises';
import {homedir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

const root = fileURLToPath(new URL('../', import.meta.url));
const storage = path.join(root, '.tools');
const destination = path.join(storage, 'tetanes-core');
const packageName = 'tetanes-core-0.12.2';
const checksum = '5ab83febf2a67da4ec29d509e9848c18109a7a564f0472a98cdc5cea7c701f47';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function run(command, args) {
    const result = spawnSync(command, args, {cwd: root, stdio: 'inherit', windowsHide: true});
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`${command} failed (${result.status})`);
}
await mkdir(storage, {recursive: true});
if ((await lstat(storage)).isSymbolicLink()
    || await realpath(storage) !== path.join(await realpath(root), '.tools')) {
    throw new Error('Build storage must be a real directory inside the project');
}
const cache = path.join(process.env.CARGO_HOME || path.join(homedir(), '.cargo'), 'registry', 'cache');
let archive;
if (existsSync(cache)) {
    for (const entry of await readdir(cache)) {
        const candidate = path.join(cache, entry, `${packageName}.crate`);
        if (existsSync(candidate)) { archive = await readFile(candidate); break; }
    }
}
if (!archive) {
    const response = await fetch(`https://static.crates.io/crates/tetanes-core/${packageName}.crate`);
    if (!response.ok) throw new Error(`Upstream download failed (${response.status})`);
    archive = Buffer.from(await response.arrayBuffer());
}
if (hash(archive) !== checksum) throw new Error('Upstream package checksum mismatch');
const staging = await mkdtemp(path.join(storage, 'core-prepare-'));
const archivePath = path.join(staging, 'upstream.crate');
await writeFile(archivePath, archive);
// Extraction is permitted only after verifying this exact known upstream archive.
run('tar', ['-xzf', archivePath, '-C', staging]);
const extracted = path.join(staging, packageName);
const patch = path.join(root, 'patches', `${packageName}.patch`);
const directory = path.relative(root, extracted).split(path.sep).join('/');
run('git', ['apply', '--check', `--directory=${directory}`, patch]);
run('git', ['apply', `--directory=${directory}`, patch]);
// Only replace the fixed generated dependency folder, never a user-provided path
// or symlink. Failed extraction/application leaves the working dependency intact.
if (existsSync(destination)) {
    if ((await lstat(destination)).isSymbolicLink()) throw new Error('Refusing to replace a linked dependency directory');
    if (path.dirname(path.resolve(destination)) !== path.resolve(storage)) throw new Error('Invalid dependency destination');
    await rm(destination, {recursive: true});
}
await rename(extracted, destination);
await rm(staging, {recursive: true});
console.log('Prepared tetanes-core 0.12.2 from verified upstream source and local patch.');
