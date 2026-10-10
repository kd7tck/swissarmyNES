import {mkdir, access, readFile, writeFile} from 'node:fs/promises';
import {dirname, resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {buildAssets} from './assets.mjs';

const sourceDirectory = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
if (args.length !== 0 && (args.length !== 2 || args[0] !== '--project')) {
  throw new Error('Usage: node examples/platform-demo/install.mjs [--project NAME]');
}
const name = args[1] ?? 'platform-demo';
if (!/^[A-Za-z0-9_-]+$/.test(name)) throw new Error('Project name must contain letters, digits, _ or -');
const projects = resolve(sourceDirectory, '..', '..', 'projects');
const target = resolve(projects, name);
try {
  await access(target);
  process.stdout.write(`Project ${name} already exists; its files were preserved.\n`);
} catch (error) {
  if (error.code !== 'ENOENT') throw error;
  await mkdir(projects, {recursive: true});
  await mkdir(target);
  const now = Math.floor(Date.now() / 1000);
  for (const filename of ['main.swiss', 'rooms.swiss']) {
    await writeFile(resolve(target, filename), await readFile(resolve(sourceDirectory, filename)), {flag: 'wx'});
  }
  await writeFile(resolve(target, 'assets.json'), JSON.stringify(buildAssets()), {flag: 'wx'});
  await writeFile(resolve(target, 'project.json'), JSON.stringify({name, created_at: now, modified_at: now}), {flag: 'wx'});
  process.stdout.write(`Installed editable project ${name}. Open it in SwissArmyNES and press Run.\n`);
}
