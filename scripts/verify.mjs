// Portable, serialized release checks; generated artifacts remain ignored.
import {spawnSync} from 'node:child_process';
import {existsSync, readdirSync} from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const flags = new Set(process.argv.slice(2));
for (const flag of flags) {
    if (!['--frontend', '--native', '--offline'].includes(flag)) throw new Error(`Unknown option ${flag}`);
}
if (flags.has('--frontend') && flags.has('--native')) throw new Error('Choose one verification mode');
function run(command, args) {
    const result = spawnSync(command, args, {cwd: root, stdio: 'inherit', windowsHide: true});
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`${command} failed (${result.status})`);
}
const frontendTests = readdirSync(path.join(root, 'tests/js'))
    .filter(name => name.endsWith('.test.cjs') && name !== 'wasm.test.cjs')
    .sort().map(name => path.join('tests/js', name));
if (!flags.has('--frontend')) {
    run(process.execPath, ['scripts/fetch-test-fixtures.mjs']);
    run(process.execPath, ['scripts/prepare-core.mjs']);
    const locked = ['--locked', ...(flags.has('--offline') ? ['--offline'] : [])];
    run('cargo', ['fmt', '--all', '--', '--check']);
    run('cargo', ['test', '--workspace', '--all-targets', ...locked]);
    run('cargo', ['clippy', '--workspace', '--all-targets', ...locked, '--', '-D', 'warnings']);
    if (!flags.has('--native')) {
        run('cargo', ['build', '-p', 'swiss-emulator', '--release', '--target', 'wasm32-unknown-unknown', ...locked]);
        const localGenerator = path.join(root, '.tools/bin', process.platform === 'win32' ? 'wasm-bindgen.exe' : 'wasm-bindgen');
        const generator = existsSync(localGenerator) ? localGenerator : 'wasm-bindgen';
        const version = spawnSync(generator, ['--version'], {cwd: root, encoding: 'utf8', windowsHide: true});
        if (version.error) throw version.error;
        if (version.status !== 0 || !/wasm-bindgen 0\.2\.106\b/.test(version.stdout)) {
            throw new Error('Install wasm-bindgen-cli 0.2.106 to match Cargo.lock');
        }
        run(generator, ['--target', 'web', '--out-dir', 'static/wasm', '--out-name', 'swiss_emulator',
            'target/wasm32-unknown-unknown/release/swiss_emulator.wasm']);
        frontendTests.push('tests/js/wasm.test.cjs');
    }
}
run(process.execPath, ['--test', ...frontendTests]);
